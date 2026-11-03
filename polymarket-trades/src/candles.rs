//! 1-minute OHLCV fragments per outcome token, one per token per block. No store:
//! the sink merges fragments of the same bucket (merge rule on `Candle` in the proto).

use std::collections::BTreeMap;
use std::str::FromStr;

use substreams::errors::Error;
use substreams::scalar::BigInt;

use crate::pb::polymarket::trades::v1 as proto;
use crate::{format_micros, price_micros};

/// Candle bucket width in seconds.
const BUCKET_SECONDS: u64 = 60;

#[substreams::handlers::map]
pub fn map_candles(trades: proto::Trades) -> Result<proto::Candles, Error> {
    substreams::skip_empty_output();
    Ok(build_candles(&trades))
}

struct Agg {
    bucket_start: u64,
    block_number: u64,
    open: BigInt,
    high: BigInt,
    low: BigInt,
    close: BigInt,
    open_log_index: u64,
    close_log_index: u64,
    shares: BigInt,
    collateral: BigInt,
    trades: u64,
}

/// One candle per token with `is_volume` trades in the block. Trades arrive in
/// log-index order, so the first seen is the open and the last the close.
fn build_candles(trades: &proto::Trades) -> proto::Candles {
    let mut by_token: BTreeMap<&str, Agg> = BTreeMap::new();
    for trade in trades.trades.iter().filter(|t| t.is_volume) {
        let Some(tx) = trade.tx.as_option() else {
            continue;
        };
        let (Ok(shares), Ok(collateral)) = (
            BigInt::from_str(&trade.shares),
            BigInt::from_str(&trade.collateral),
        ) else {
            continue;
        };
        if shares.is_zero() {
            continue;
        }
        let price = price_micros(&collateral, &shares);
        match by_token.get_mut(trade.token_id.as_str()) {
            Some(agg) => {
                if price > agg.high {
                    agg.high = price.clone();
                }
                if price < agg.low {
                    agg.low = price.clone();
                }
                agg.close = price;
                agg.close_log_index = tx.log_index;
                agg.shares = agg.shares.clone() + shares;
                agg.collateral = agg.collateral.clone() + collateral;
                agg.trades += 1;
            }
            None => {
                by_token.insert(
                    trade.token_id.as_str(),
                    Agg {
                        bucket_start: tx.timestamp - tx.timestamp % BUCKET_SECONDS,
                        block_number: tx.block_number,
                        open: price.clone(),
                        high: price.clone(),
                        low: price.clone(),
                        close: price,
                        open_log_index: tx.log_index,
                        close_log_index: tx.log_index,
                        shares,
                        collateral,
                        trades: 1,
                    },
                );
            }
        }
    }
    let candles = by_token
        .into_iter()
        .map(|(token, a)| proto::Candle {
            token_id: token.to_owned(),
            bucket_start: a.bucket_start,
            open: format_micros(&a.open),
            high: format_micros(&a.high),
            low: format_micros(&a.low),
            close: format_micros(&a.close),
            volume_shares: a.shares.to_string(),
            volume_collateral: a.collateral.to_string(),
            trades: a.trades,
            block_number: a.block_number,
            open_log_index: a.open_log_index,
            close_log_index: a.close_log_index,
        })
        .collect();
    proto::Candles { candles }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn trade(token: &str, shares: u64, collateral: u64, is_volume: bool, log: u64) -> proto::Trade {
        proto::Trade {
            token_id: token.to_owned(),
            shares: shares.to_string(),
            collateral: collateral.to_string(),
            is_volume,
            tx: Some(proto::TransactionContext {
                block_number: 100,
                timestamp: 1_700_000_059,
                log_index: log,
                ..Default::default()
            })
            .into(),
            ..Default::default()
        }
    }

    #[test]
    fn ohlcv_over_several_matches_of_one_token() {
        let trades = proto::Trades {
            trades: vec![
                trade("A", 100, 40, true, 3),  // open 0.40
                trade("A", 100, 90, false, 4), // maker fill: not volume, ignored
                trade("B", 10, 5, true, 5),
                trade("A", 200, 110, true, 6), // high 0.55
                trade("A", 300, 105, true, 8), // low 0.35
                trade("A", 100, 45, true, 9),  // close 0.45
            ],
        };
        let c = build_candles(&trades).candles;
        assert_eq!(c.len(), 2);
        let a = &c[0];
        assert_eq!(a.token_id, "A");
        assert_eq!(a.bucket_start, 1_700_000_040);
        assert_eq!(
            (a.open.as_str(), a.high.as_str(), a.low.as_str(), a.close.as_str()),
            ("0.400000", "0.550000", "0.350000", "0.450000")
        );
        assert_eq!(a.volume_shares, "700");
        assert_eq!(a.volume_collateral, "300");
        assert_eq!(a.trades, 4);
        assert_eq!((a.open_log_index, a.close_log_index), (3, 9));
        assert_eq!(a.block_number, 100);
        assert_eq!(c[1].token_id, "B");
        assert_eq!(c[1].open, c[1].close);
    }

    #[test]
    fn no_volume_trades_no_candles() {
        let trades = proto::Trades {
            trades: vec![trade("A", 100, 40, false, 1)],
        };
        assert!(build_candles(&trades).candles.is_empty());
    }
}
