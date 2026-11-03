//! Per-order fill progress, without stores: every order filled in a block gets one
//! `OrderFill` with that block's fills. A sink keeps the running total per
//! `order_hash` by summing fragments (merge rule on `OrderFill` in the proto). A
//! store per order would hold one key set for every order ever filled (about 2B
//! keys from 84902353 at 2026 rates), while 94.5% of orders finish in one block.

use std::collections::BTreeMap;
use std::str::FromStr;

use substreams::errors::Error;
use substreams::scalar::BigInt;

use polymarket_substreams_common::format_address;

use crate::pb::polymarket::trades::v1 as proto;

#[substreams::handlers::map]
pub fn map_order_fills(trades: proto::Trades) -> Result<proto::OrderFills, Error> {
    substreams::skip_empty_output();
    Ok(build_order_fills(&trades))
}

struct Agg<'a> {
    trade: &'a proto::Trade,
    shares: BigInt,
    collateral: BigInt,
    fees: BigInt,
    fills: u64,
}

fn amount(value: &str) -> BigInt {
    BigInt::from_str(value).unwrap_or_else(|_| BigInt::zero())
}

/// One `OrderFill` per order filled in the block, ordered by order hash.
fn build_order_fills(trades: &proto::Trades) -> proto::OrderFills {
    let mut orders: BTreeMap<&[u8], Agg> = BTreeMap::new();
    for trade in &trades.trades {
        let agg = orders.entry(trade.order_hash.as_slice()).or_insert_with(|| Agg {
            trade,
            shares: BigInt::zero(),
            collateral: BigInt::zero(),
            fees: BigInt::zero(),
            fills: 0,
        });
        agg.shares = agg.shares.clone() + amount(&trade.shares);
        agg.collateral = agg.collateral.clone() + amount(&trade.collateral);
        agg.fees = agg.fees.clone() + amount(&trade.fee);
        agg.fills += 1;
    }
    let orders = orders
        .into_iter()
        .map(|(hash, a)| {
            let (block_number, timestamp) = a
                .trade
                .tx
                .as_option()
                .map_or((0, 0), |tx| (tx.block_number, tx.timestamp));
            proto::OrderFill {
                order_hash: format_address(hash),
                owner: a.trade.maker.clone(),
                token_id: a.trade.token_id.clone(),
                side: a.trade.side,
                exchange: a.trade.exchange.clone(),
                shares: a.shares.to_string(),
                collateral: a.collateral.to_string(),
                fees: a.fees.to_string(),
                fills: a.fills,
                block_number,
                timestamp,
            }
        })
        .collect();
    proto::OrderFills { orders }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use std::collections::HashMap;

    /// Running totals per order, merged from per-block fragments the way the sink
    /// does: amounts and fills summed, first block = min, last block = max.
    #[derive(Default, Debug, PartialEq)]
    pub(crate) struct Totals {
        pub(crate) shares: u64,
        pub(crate) collateral: u64,
        pub(crate) fees: u64,
        pub(crate) fills: u64,
        pub(crate) first_block: u64,
        pub(crate) last_block: u64,
    }

    pub(crate) fn merge(blocks: &[&proto::Trades]) -> HashMap<String, Totals> {
        let mut out: HashMap<String, Totals> = HashMap::new();
        for b in blocks {
            for o in build_order_fills(b).orders {
                let t = out.entry(o.order_hash).or_insert(Totals {
                    first_block: u64::MAX,
                    ..Default::default()
                });
                t.shares += o.shares.parse::<u64>().expect("amount");
                t.collateral += o.collateral.parse::<u64>().expect("amount");
                t.fees += o.fees.parse::<u64>().expect("amount");
                t.fills += o.fills;
                t.first_block = t.first_block.min(o.block_number);
                t.last_block = t.last_block.max(o.block_number);
            }
        }
        out
    }

    fn fill(hash: u8, shares: u64, collateral: u64, fee: u64, block: u64) -> proto::Trade {
        proto::Trade {
            order_hash: vec![hash; 32],
            maker: "0xowner".to_owned(),
            token_id: "T".to_owned(),
            side: 1,
            exchange: "ctf_v2".to_owned(),
            shares: shares.to_string(),
            collateral: collateral.to_string(),
            fee: fee.to_string(),
            tx: Some(proto::TransactionContext {
                block_number: block,
                timestamp: block * 2,
                ..Default::default()
            })
            .into(),
            ..Default::default()
        }
    }

    #[test]
    fn partial_fills_sum_across_blocks() {
        // Order 0xaa.. fills twice in block 10 and once in block 12; 0xbb.. once in 12.
        let b10 = proto::Trades {
            trades: vec![fill(0xaa, 100, 40, 1, 10), fill(0xaa, 50, 20, 0, 10)],
        };
        let b12 = proto::Trades {
            trades: vec![fill(0xbb, 10, 5, 0, 12), fill(0xaa, 25, 11, 2, 12)],
        };

        // Each block emits that block's fills only.
        let o = build_order_fills(&b10).orders;
        assert_eq!(o.len(), 1);
        assert_eq!((o[0].shares.as_str(), o[0].fills), ("150", 2));
        assert_eq!((o[0].block_number, o[0].timestamp), (10, 20));
        let o = build_order_fills(&b12).orders;
        assert_eq!(o.len(), 2);
        assert_eq!(o[0].order_hash, format!("0x{}", "aa".repeat(32)));
        assert_eq!((o[0].shares.as_str(), o[0].fills), ("25", 1));
        assert_eq!((o[0].owner.as_str(), o[0].token_id.as_str(), o[0].side), ("0xowner", "T", 1));

        // Merged across blocks, as the sink does.
        let totals = merge(&[&b10, &b12]);
        assert_eq!(
            totals[&format!("0x{}", "aa".repeat(32))],
            Totals {
                shares: 175,
                collateral: 71,
                fees: 3,
                fills: 3,
                first_block: 10,
                last_block: 12
            }
        );
        let b = &totals[&format!("0x{}", "bb".repeat(32))];
        assert_eq!((b.fills, b.first_block, b.last_block), (1, 12, 12));
    }
}
