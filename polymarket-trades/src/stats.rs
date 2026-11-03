//! Per-token market stats: running totals and last price in stores, and a map that
//! emits each traded token's summary for the block.

use std::collections::BTreeMap;
use std::str::FromStr;

use substreams::errors::Error;
use substreams::scalar::BigInt;
use substreams::store::{
    StoreAdd, StoreAddBigInt, StoreGet, StoreGetBigInt, StoreGetString, StoreNew, StoreSet,
    StoreSetString,
};

use polymarket_substreams_common::bigint_to_u64;

use crate::pb::polymarket::trades::v1 as proto;
use crate::{format_price, SIDE_BUY};

// Fields of `store_token_totals`, keyed `<token_id>:<field>`.
const TRADES: &str = "trades";
const BUYS: &str = "buys";
const SELLS: &str = "sells";
const VOLUME_COLLATERAL: &str = "volume_collateral";
const VOLUME_SHARES: &str = "volume_shares";
const FEES: &str = "fees";

fn key(token_id: &str, field: &str) -> String {
    let mut k = String::with_capacity(token_id.len() + 1 + field.len());
    k.push_str(token_id);
    k.push(':');
    k.push_str(field);
    k
}

/// Ordinal of a trade's store writes: its block-wide log index.
fn ordinal(trade: &proto::Trade) -> u64 {
    trade.tx.as_option().map_or(0, |tx| tx.log_index)
}

/// The `(field, amount)` additions one trade makes to its token's totals. Volume
/// fields count only `is_volume` trades (each match once per token); fees count
/// every fill.
fn total_additions(trade: &proto::Trade) -> Vec<(&'static str, BigInt)> {
    let mut out = Vec::with_capacity(6);
    if let Ok(fee) = BigInt::from_str(&trade.fee) {
        if !fee.is_zero() {
            out.push((FEES, fee));
        }
    }
    if trade.is_volume {
        let (Ok(collateral), Ok(shares)) = (
            BigInt::from_str(&trade.collateral),
            BigInt::from_str(&trade.shares),
        ) else {
            return out;
        };
        out.push((TRADES, BigInt::one()));
        out.push((if trade.side == SIDE_BUY { BUYS } else { SELLS }, BigInt::one()));
        out.push((VOLUME_COLLATERAL, collateral));
        out.push((VOLUME_SHARES, shares));
    }
    out
}

#[substreams::handlers::store]
pub fn store_token_totals(trades: proto::Trades, store: StoreAddBigInt) {
    for trade in &trades.trades {
        let ord = ordinal(trade);
        for (field, amount) in total_additions(trade) {
            store.add(ord, key(&trade.token_id, field), amount);
        }
    }
}

#[substreams::handlers::store]
pub fn store_last_price(trades: proto::Trades, store: StoreSetString) {
    for trade in trades.trades.iter().filter(|t| t.is_volume) {
        store.set(ordinal(trade), &trade.token_id, &trade.price);
    }
}

/// Read access to the two stores, so `build_stats` runs natively in tests.
trait StatsSource {
    fn total(&self, token_id: &str, field: &str) -> BigInt;
    fn last_price(&self, token_id: &str) -> Option<String>;
}

struct Stores {
    totals: StoreGetBigInt,
    last_price: StoreGetString,
}

impl StatsSource for Stores {
    fn total(&self, token_id: &str, field: &str) -> BigInt {
        self.totals
            .get_last(key(token_id, field))
            .unwrap_or_else(BigInt::zero)
    }
    fn last_price(&self, token_id: &str) -> Option<String> {
        self.last_price.get_last(token_id)
    }
}

#[substreams::handlers::map]
pub fn map_market_stats(
    trades: proto::Trades,
    totals: StoreGetBigInt,
    last_price: StoreGetString,
) -> Result<proto::MarketStats, Error> {
    substreams::skip_empty_output();
    Ok(build_stats(&trades, &Stores { totals, last_price }))
}

#[derive(Default)]
struct BlockAgg {
    collateral: BigInt,
    shares: BigInt,
    block: u64,
    timestamp: u64,
}

/// One `TokenStats` per token traded in the block, ordered by token id. The stores
/// already include this block's writes.
fn build_stats(trades: &proto::Trades, src: &impl StatsSource) -> proto::MarketStats {
    let mut block: BTreeMap<&str, BlockAgg> = BTreeMap::new();
    for trade in &trades.trades {
        let agg = block.entry(trade.token_id.as_str()).or_default();
        if let Some(tx) = trade.tx.as_option() {
            agg.block = tx.block_number;
            agg.timestamp = tx.timestamp;
        }
        if trade.is_volume {
            if let (Ok(c), Ok(s)) = (
                BigInt::from_str(&trade.collateral),
                BigInt::from_str(&trade.shares),
            ) {
                agg.collateral = agg.collateral.clone() + c;
                agg.shares = agg.shares.clone() + s;
            }
        }
    }
    // `BigInt::to_u64` panics out of range; the shared helper doesn't.
    let count = |token: &str, field: &str| bigint_to_u64(&src.total(token, field));
    let tokens = block
        .into_iter()
        .map(|(token, agg)| proto::TokenStats {
            token_id: token.to_owned(),
            trades: count(token, TRADES),
            buys: count(token, BUYS),
            sells: count(token, SELLS),
            volume_collateral: src.total(token, VOLUME_COLLATERAL).to_string(),
            volume_shares: src.total(token, VOLUME_SHARES).to_string(),
            fees: src.total(token, FEES).to_string(),
            last_price: src.last_price(token).unwrap_or_default(),
            block_vwap: if agg.shares.is_zero() {
                String::new()
            } else {
                format_price(&agg.collateral, &agg.shares)
            },
            block_volume_collateral: agg.collateral.to_string(),
            block_volume_shares: agg.shares.to_string(),
            last_trade_block: agg.block,
            last_trade_timestamp: agg.timestamp,
        })
        .collect();
    proto::MarketStats { tokens }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use std::collections::HashMap;

    /// In-memory stand-in for the two stores: applies blocks the way the runtime
    /// does (adds and last-write-wins sets, in ordinal order).
    #[derive(Default)]
    pub(crate) struct MemStores {
        totals: HashMap<String, BigInt>,
        last_price: HashMap<String, String>,
    }

    impl MemStores {
        pub(crate) fn apply(&mut self, trades: &proto::Trades) {
            for trade in &trades.trades {
                for (field, amount) in total_additions(trade) {
                    let e = self
                        .totals
                        .entry(key(&trade.token_id, field))
                        .or_insert_with(BigInt::zero);
                    *e = e.clone() + amount;
                }
                if trade.is_volume {
                    self.last_price
                        .insert(trade.token_id.clone(), trade.price.clone());
                }
            }
        }
    }

    impl StatsSource for MemStores {
        fn total(&self, token_id: &str, field: &str) -> BigInt {
            self.totals
                .get(&key(token_id, field))
                .cloned()
                .unwrap_or_else(BigInt::zero)
        }
        fn last_price(&self, token_id: &str) -> Option<String> {
            self.last_price.get(token_id).cloned()
        }
    }

    pub(crate) fn stats_after(blocks: &[&proto::Trades]) -> proto::MarketStats {
        let mut mem = MemStores::default();
        for b in blocks {
            mem.apply(b);
        }
        build_stats(blocks[blocks.len() - 1], &mem)
    }

    fn trade(
        token: &str,
        side: u32,
        shares: u64,
        collateral: u64,
        fee: u64,
        is_volume: bool,
        block: u64,
    ) -> proto::Trade {
        proto::Trade {
            token_id: token.to_owned(),
            side,
            shares: shares.to_string(),
            collateral: collateral.to_string(),
            price: format_price(&BigInt::from(collateral), &BigInt::from(shares)),
            fee: fee.to_string(),
            is_volume,
            is_match: is_volume,
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
    fn volume_counts_each_match_once_and_fees_every_fill() {
        // Block 1: one match on token A. Two maker SELL fills (not volume: the taker
        // fill covers them) and the taker BUY of 300 shares for 150, fee 3.
        let b1 = proto::Trades {
            trades: vec![
                trade("A", 1, 100, 50, 1, false, 1),
                trade("A", 1, 200, 100, 0, false, 1),
                trade("A", SIDE_BUY, 300, 150, 3, true, 1),
            ],
        };
        // Block 2: a taker SELL of 100 shares on A for 60.
        let b2 = proto::Trades {
            trades: vec![trade("A", 1, 100, 60, 0, true, 2)],
        };
        let s = stats_after(&[&b1]);
        assert_eq!(s.tokens.len(), 1);
        let a = &s.tokens[0];
        // Volume is the sum of is_volume (here: is_match) collateral: 150, not 300.
        assert_eq!(a.volume_collateral, "150");
        assert_eq!(a.volume_shares, "300");
        assert_eq!((a.trades, a.buys, a.sells), (1, 1, 0));
        assert_eq!(a.fees, "4");
        assert_eq!(a.last_price, "0.500000");
        assert_eq!(a.block_vwap, "0.500000");

        let s = stats_after(&[&b1, &b2]);
        let a = &s.tokens[0];
        assert_eq!(a.volume_collateral, "210");
        assert_eq!(a.volume_shares, "400");
        assert_eq!((a.trades, a.buys, a.sells), (2, 1, 1));
        assert_eq!(a.last_price, "0.600000");
        // Block fields cover only block 2.
        assert_eq!(a.block_volume_collateral, "60");
        assert_eq!(a.block_vwap, "0.600000");
        assert_eq!((a.last_trade_block, a.last_trade_timestamp), (2, 4));
    }

    #[test]
    fn token_with_only_non_volume_fills_still_listed() {
        let b = proto::Trades {
            trades: vec![trade("B", 1, 10, 5, 0, false, 7)],
        };
        let s = stats_after(&[&b]);
        assert_eq!(s.tokens[0].token_id, "B");
        assert_eq!(s.tokens[0].volume_collateral, "0");
        assert_eq!(s.tokens[0].block_vwap, "");
        assert_eq!(s.tokens[0].last_price, "");
    }
}
