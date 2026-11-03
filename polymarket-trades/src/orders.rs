//! Per-order fill progress: cumulative amounts per `order_hash` in stores, and a map
//! that emits every order filled in the block with its running totals.

use std::collections::BTreeMap;
use std::str::FromStr;

use substreams::errors::Error;
use substreams::scalar::BigInt;
use substreams::store::{
    StoreAdd, StoreAddBigInt, StoreGet, StoreGetBigInt, StoreGetInt64, StoreNew,
    StoreSetIfNotExists, StoreSetIfNotExistsInt64,
};

use polymarket_substreams_common::{bigint_to_u64, format_address};

use crate::pb::polymarket::trades::v1 as proto;

// Fields of `store_order_filled`, keyed `<order_hash>:<field>`.
const SHARES: &str = "shares";
const COLLATERAL: &str = "collateral";
const FEES: &str = "fees";
const FILLS: &str = "fills";

fn key(order_hash: &str, field: &str) -> String {
    let mut k = String::with_capacity(order_hash.len() + 1 + field.len());
    k.push_str(order_hash);
    k.push(':');
    k.push_str(field);
    k
}

fn tx(trade: &proto::Trade) -> (u64, u64) {
    trade
        .tx
        .as_option()
        .map_or((0, 0), |tx| (tx.log_index, tx.block_number))
}

/// The `(field, amount)` additions one fill makes to its order's totals.
fn fill_additions(trade: &proto::Trade) -> Vec<(&'static str, BigInt)> {
    let mut out = Vec::with_capacity(4);
    for (field, value) in [
        (SHARES, &trade.shares),
        (COLLATERAL, &trade.collateral),
        (FEES, &trade.fee),
    ] {
        if let Ok(v) = BigInt::from_str(value) {
            if !v.is_zero() {
                out.push((field, v));
            }
        }
    }
    out.push((FILLS, BigInt::one()));
    out
}

#[substreams::handlers::store]
pub fn store_order_filled(trades: proto::Trades, store: StoreAddBigInt) {
    for trade in &trades.trades {
        let hash = format_address(&trade.order_hash);
        let (ord, _) = tx(trade);
        for (field, amount) in fill_additions(trade) {
            store.add(ord, key(&hash, field), amount);
        }
    }
}

/// Block of each order's first fill, keyed by order hash.
#[substreams::handlers::store]
pub fn store_order_first_seen(trades: proto::Trades, store: StoreSetIfNotExistsInt64) {
    for trade in &trades.trades {
        let (ord, block) = tx(trade);
        store.set_if_not_exists(ord, format_address(&trade.order_hash), &(block as i64));
    }
}

/// Read access to the two stores, so `build_order_fills` runs natively in tests.
trait OrderSource {
    fn total(&self, order_hash: &str, field: &str) -> BigInt;
    fn first_seen(&self, order_hash: &str) -> Option<u64>;
}

struct Stores {
    filled: StoreGetBigInt,
    first_seen: StoreGetInt64,
}

impl OrderSource for Stores {
    fn total(&self, order_hash: &str, field: &str) -> BigInt {
        self.filled
            .get_last(key(order_hash, field))
            .unwrap_or_else(BigInt::zero)
    }
    fn first_seen(&self, order_hash: &str) -> Option<u64> {
        self.first_seen
            .get_last(order_hash)
            .and_then(|b| u64::try_from(b).ok())
    }
}

#[substreams::handlers::map]
pub fn map_order_fills(
    trades: proto::Trades,
    filled: StoreGetBigInt,
    first_seen: StoreGetInt64,
) -> Result<proto::OrderFills, Error> {
    substreams::skip_empty_output();
    Ok(build_order_fills(&trades, &Stores { filled, first_seen }))
}

/// One `OrderFill` per order filled in the block, ordered by order hash. The stores
/// already include this block's writes.
fn build_order_fills(trades: &proto::Trades, src: &impl OrderSource) -> proto::OrderFills {
    let mut orders: BTreeMap<String, (&proto::Trade, u64)> = BTreeMap::new();
    for trade in &trades.trades {
        orders
            .entry(format_address(&trade.order_hash))
            .and_modify(|(_, n)| *n += 1)
            .or_insert((trade, 1));
    }
    let orders = orders
        .into_iter()
        .map(|(hash, (trade, block_fills))| {
            let (_, block) = tx(trade);
            proto::OrderFill {
                filled_shares: src.total(&hash, SHARES).to_string(),
                filled_collateral: src.total(&hash, COLLATERAL).to_string(),
                fees: src.total(&hash, FEES).to_string(),
                fills: bigint_to_u64(&src.total(&hash, FILLS)),
                first_fill_block: src.first_seen(&hash).unwrap_or(block),
                order_hash: hash,
                owner: trade.maker.clone(),
                token_id: trade.token_id.clone(),
                side: trade.side,
                exchange: trade.exchange.clone(),
                last_fill_block: block,
                block_fills,
            }
        })
        .collect();
    proto::OrderFills { orders }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use std::collections::HashMap;

    /// In-memory stand-in for the two stores, applied the way the runtime does.
    #[derive(Default)]
    pub(crate) struct MemStores {
        totals: HashMap<String, BigInt>,
        first_seen: HashMap<String, u64>,
    }

    impl MemStores {
        pub(crate) fn apply(&mut self, trades: &proto::Trades) {
            for trade in &trades.trades {
                let hash = format_address(&trade.order_hash);
                for (field, amount) in fill_additions(trade) {
                    let e = self
                        .totals
                        .entry(key(&hash, field))
                        .or_insert_with(BigInt::zero);
                    *e = e.clone() + amount;
                }
                self.first_seen.entry(hash).or_insert(tx(trade).1);
            }
        }
    }

    impl OrderSource for MemStores {
        fn total(&self, order_hash: &str, field: &str) -> BigInt {
            self.totals
                .get(&key(order_hash, field))
                .cloned()
                .unwrap_or_else(BigInt::zero)
        }
        fn first_seen(&self, order_hash: &str) -> Option<u64> {
            self.first_seen.get(order_hash).copied()
        }
    }

    pub(crate) fn fills_after(blocks: &[&proto::Trades]) -> proto::OrderFills {
        let mut mem = MemStores::default();
        for b in blocks {
            mem.apply(b);
        }
        build_order_fills(blocks[blocks.len() - 1], &mem)
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

        let o = fills_after(&[&b10]).orders;
        assert_eq!(o.len(), 1);
        assert_eq!((o[0].filled_shares.as_str(), o[0].fills), ("150", 2));

        let o = fills_after(&[&b10, &b12]).orders;
        assert_eq!(o.len(), 2);
        let a = &o[0];
        assert_eq!(a.order_hash, format!("0x{}", "aa".repeat(32)));
        assert_eq!(a.filled_shares, "175");
        assert_eq!(a.filled_collateral, "71");
        assert_eq!(a.fees, "3");
        assert_eq!(a.fills, 3);
        assert_eq!(a.block_fills, 1);
        assert_eq!((a.first_fill_block, a.last_fill_block), (10, 12));
        assert_eq!((a.owner.as_str(), a.token_id.as_str(), a.side), ("0xowner", "T", 1));
        let b = &o[1];
        assert_eq!((b.fills, b.first_fill_block, b.last_fill_block), (1, 12, 12));
        assert_eq!(b.fees, "0");
    }
}
