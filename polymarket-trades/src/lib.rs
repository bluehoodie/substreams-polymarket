//! Polymarket trade tape: one `Trade` per `OrderFilled`, read from the outputs of the
//! sibling exchange packages (no block decoding here).

#[allow(dead_code, clippy::all)]
pub mod pb;
mod candles;
mod db;
mod orders;
mod stats;

use std::str::FromStr;

use substreams::errors::Error;
use substreams::scalar::BigInt;

use pb::polymarket::exchange::v1 as exchange;
use pb::polymarket::neg_risk_ctf::v1 as neg_risk;
use pb::polymarket::trades::v1 as proto;

/// `exchange` label of trades from the Neg Risk CTF Exchange V2 (the exchange
/// package labels its own events `ctf_v2` / `exchange_v3`).
const NEG_RISK_V2_LABEL: &str = "neg_risk_v2";

// The upstream packages emit addresses through `format_address` ("0x" + lowercase
// hex), so the taker is compared with each exchange's address in that same form:
// a byte comparison of two short strings, with no decoding or allocation.
const CTF_EXCHANGE_FORMATTED: &str = "0xe111180000d2663c0091e4f400237545b87b996b";
const NEG_RISK_CTF_FORMATTED: &str = "0xe2222d279d744050d28e00520010520000310f59";
const EXCHANGE_V3_FORMATTED: &str = "0xe3333700ca9d93003f00f0f71f8515005f6c00aa";

/// Order side, as emitted in `OrderFilled.side`.
pub(crate) const SIDE_BUY: u32 = 0;

/// 10^6: prices are collateral per share with 6 decimals.
const PRICE_SCALE: u64 = 1_000_000;

#[substreams::handlers::map]
pub fn map_trades(
    exchange_events: exchange::ExchangeEvents,
    neg_risk_events: neg_risk::TradingEvents,
) -> Result<proto::Trades, Error> {
    substreams::skip_empty_output();

    let mut trades =
        Vec::with_capacity(exchange_events.order_filled.len() + neg_risk_events.order_filled.len());
    for fill in exchange_events.order_filled {
        let exchange = fill.exchange;
        let fill = Fill {
            order_hash: fill.order_hash,
            maker: fill.maker,
            taker: fill.taker,
            side: fill.side,
            token_id: fill.token_id,
            maker_amount_filled: fill.maker_amount_filled,
            taker_amount_filled: fill.taker_amount_filled,
            fee: fill.fee,
            tx: fill.tx.into_option().map(|tx| proto::TransactionContext {
                tx_hash: tx.tx_hash,
                log_index: tx.log_index,
                block_number: tx.block_number,
                timestamp: tx.timestamp,
            }),
        };
        if let Some(trade) = to_trade(fill, exchange) {
            trades.push(trade);
        }
    }
    for fill in neg_risk_events.order_filled {
        let fill = Fill {
            order_hash: fill.order_hash,
            maker: fill.maker,
            taker: fill.taker,
            side: fill.side,
            token_id: fill.token_id,
            maker_amount_filled: fill.maker_amount_filled,
            taker_amount_filled: fill.taker_amount_filled,
            fee: fill.fee,
            tx: fill.tx.into_option().map(|tx| proto::TransactionContext {
                tx_hash: tx.tx_hash,
                log_index: tx.log_index,
                block_number: tx.block_number,
                timestamp: tx.timestamp,
            }),
        };
        if let Some(trade) = to_trade(fill, NEG_RISK_V2_LABEL.to_owned()) {
            trades.push(trade);
        }
    }

    // Both inputs are in block order; merge them by the block-wide log index.
    trades.sort_by_key(|t| t.tx.as_option().map_or(0, |tx| tx.log_index));
    mark_volume(&mut trades);
    Ok(proto::Trades { trades })
}

/// Sets `is_volume`, so that summing `is_volume` trades counts each match once per
/// token.
///
/// A match settles in one transaction: the exchange emits each maker order's
/// `OrderFilled`, then the taker order's own fill (`is_match`). Maker fills on the
/// taker's token are the other side of the taker fill, so only the taker fill counts.
/// Maker fills on another token (the complement, in a mint or merge match) are trades
/// of their own token and count. Fills not followed by a taker fill in the same
/// transaction and exchange (direct `fillOrder`) count too. `trades` must be in block
/// order.
fn mark_volume(trades: &mut [proto::Trade]) {
    let mut pending: Vec<usize> = Vec::new();
    for i in 0..trades.len() {
        if let Some(&first) = pending.first() {
            if !same_settlement(&trades[first], &trades[i]) {
                // The pending fills' transaction ended without a taker fill.
                for &p in &pending {
                    trades[p].is_volume = true;
                }
                pending.clear();
            }
        }
        if trades[i].is_match {
            trades[i].is_volume = true;
            for &p in &pending {
                trades[p].is_volume = trades[p].token_id != trades[i].token_id;
            }
            pending.clear();
        } else {
            pending.push(i);
        }
    }
    for p in pending {
        trades[p].is_volume = true;
    }
}

/// Whether two fills come from the same transaction on the same exchange.
fn same_settlement(a: &proto::Trade, b: &proto::Trade) -> bool {
    a.exchange == b.exchange
        && a.tx.as_option().map(|t| &t.tx_hash) == b.tx.as_option().map(|t| &t.tx_hash)
}

/// The `OrderFilled` fields both exchange packages emit, under one type.
struct Fill {
    order_hash: Vec<u8>,
    maker: String,
    taker: String,
    side: u32,
    token_id: String,
    maker_amount_filled: String,
    taker_amount_filled: String,
    fee: String,
    tx: Option<proto::TransactionContext>,
}

/// Builds the trade for one fill, or `None` if an amount is not a decimal integer
/// (never the case for upstream output; skipped rather than panicking).
fn to_trade(fill: Fill, exchange: String) -> Option<proto::Trade> {
    let maker_amount = BigInt::from_str(&fill.maker_amount_filled).ok()?;
    let taker_amount = BigInt::from_str(&fill.taker_amount_filled).ok()?;
    let fee = BigInt::from_str(&fill.fee).ok()?;
    let amounts = trade_amounts(fill.side, maker_amount, taker_amount, &fee);
    let is_match = fill.taker == exchange_address(&exchange);
    Some(proto::Trade {
        token_id: fill.token_id,
        side: fill.side,
        shares: amounts.shares.to_string(),
        collateral: amounts.collateral.to_string(),
        price: amounts.price,
        fee: fill.fee,
        maker: fill.maker,
        taker: fill.taker,
        order_hash: fill.order_hash,
        exchange,
        is_match,
        tx: fill.tx.into(),
        // Set by `mark_volume` once the whole block is in order.
        is_volume: false,
    })
}

/// The formatted address of the exchange behind an `exchange` label.
fn exchange_address(label: &str) -> &'static str {
    match label {
        "ctf_v2" => CTF_EXCHANGE_FORMATTED,
        "exchange_v3" => EXCHANGE_V3_FORMATTED,
        NEG_RISK_V2_LABEL => NEG_RISK_CTF_FORMATTED,
        // An unknown label never matches: `is_match` stays false.
        _ => "",
    }
}

struct TradeAmounts {
    shares: BigInt,
    collateral: BigInt,
    price: String,
}

/// Shares, collateral and price of a fill from the filled order's point of view.
///
/// The fee is in collateral. A BUY order gives `maker_amount` collateral for
/// `taker_amount` shares and pays the fee on top; a SELL order gives `maker_amount`
/// shares for `taker_amount` collateral and has the fee taken out of it
/// (ctf-exchange-v2 `Trading.sol`; ExchangeV3 settles the same way).
fn trade_amounts(
    side: u32,
    maker_amount: BigInt,
    taker_amount: BigInt,
    fee: &BigInt,
) -> TradeAmounts {
    let (shares, collateral) = if side == SIDE_BUY {
        (taker_amount, maker_amount + fee)
    } else {
        (maker_amount, taker_amount - fee)
    };
    let price = format_price(&collateral, &shares);
    TradeAmounts {
        shares,
        collateral,
        price,
    }
}

/// `collateral / shares` rounded down to 6 decimals, e.g. "0.537000"; "0" when
/// there are no shares.
pub(crate) fn format_price(collateral: &BigInt, shares: &BigInt) -> String {
    if shares.is_zero() {
        return "0".to_owned();
    }
    format_micros(&price_micros(collateral, shares))
}

/// `collateral / shares` in millionths, rounded down. `shares` must be non-zero.
pub(crate) fn price_micros(collateral: &BigInt, shares: &BigInt) -> BigInt {
    (collateral.clone() * BigInt::from(PRICE_SCALE)) / shares.clone()
}

/// A price in millionths as a 6-decimal string, e.g. 537000 -> "0.537000".
pub(crate) fn format_micros(scaled: &BigInt) -> String {
    let digits = scaled.to_string();
    let (int_part, frac_part) = if digits.len() > 6 {
        digits.split_at(digits.len() - 6)
    } else {
        ("0", digits.as_str())
    };
    format!("{int_part}.{frac_part:0>6}")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn amounts(side: u32, maker: u64, taker: u64, fee: u64) -> (String, String, String) {
        let a = trade_amounts(side, maker.into(), taker.into(), &BigInt::from(fee));
        (a.shares.to_string(), a.collateral.to_string(), a.price)
    }

    #[test]
    fn buy_pays_fee_on_top_of_collateral() {
        // BUY 100 shares for 53 USDC, fee 0.5 USDC: pays 53.5 for 100 shares.
        assert_eq!(
            amounts(SIDE_BUY, 53_000_000, 100_000_000, 500_000),
            (
                "100000000".to_owned(),
                "53500000".to_owned(),
                "0.535000".to_owned()
            )
        );
    }

    #[test]
    fn sell_has_fee_taken_from_proceeds() {
        // SELL 100 shares for 53 USDC, fee 0.5 USDC: receives 52.5 for 100 shares.
        assert_eq!(
            amounts(1, 100_000_000, 53_000_000, 500_000),
            (
                "100000000".to_owned(),
                "52500000".to_owned(),
                "0.525000".to_owned()
            )
        );
    }

    #[test]
    fn price_rounds_down_and_pads() {
        assert_eq!(
            format_price(&BigInt::from(1u64), &BigInt::from(3u64)),
            "0.333333"
        );
        assert_eq!(
            format_price(&BigInt::from(5u64), &BigInt::from(1_000_000u64)),
            "0.000005"
        );
        assert_eq!(
            format_price(&BigInt::from(3u64), &BigInt::from(2u64)),
            "1.500000"
        );
        assert_eq!(format_price(&BigInt::from(3u64), &BigInt::from(0u64)), "0");
    }

    fn fill(taker: &str, log_index: u64) -> Fill {
        Fill {
            order_hash: vec![1; 32],
            maker: "0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".to_owned(),
            taker: taker.to_owned(),
            side: SIDE_BUY,
            token_id: "7".to_owned(),
            maker_amount_filled: "1000000".to_owned(),
            taker_amount_filled: "2000000".to_owned(),
            fee: "0".to_owned(),
            tx: Some(proto::TransactionContext {
                log_index,
                ..Default::default()
            }),
        }
    }

    #[test]
    fn is_match_only_when_taker_is_the_emitting_exchange() {
        let t = to_trade(fill(CTF_EXCHANGE_FORMATTED, 1), "ctf_v2".into()).expect("trade");
        assert!(t.is_match);
        let t = to_trade(fill(NEG_RISK_CTF_FORMATTED, 1), NEG_RISK_V2_LABEL.into()).expect("trade");
        assert!(t.is_match);
        let t = to_trade(fill(EXCHANGE_V3_FORMATTED, 1), "exchange_v3".into()).expect("trade");
        assert!(t.is_match);
        // Another exchange's address, or a wallet, is a maker fill.
        let t = to_trade(fill(EXCHANGE_V3_FORMATTED, 1), "ctf_v2".into()).expect("trade");
        assert!(!t.is_match);
        let t = to_trade(
            fill("0xbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb", 1),
            "ctf_v2".into(),
        )
        .expect("trade");
        assert!(!t.is_match);
    }

    fn tape(fills: &[(&str, &str, bool)]) -> Vec<proto::Trade> {
        // (tx, token, is_match) per fill, all on one exchange.
        fills
            .iter()
            .enumerate()
            .map(|(i, (tx, token, is_match))| proto::Trade {
                token_id: (*token).to_owned(),
                exchange: "ctf_v2".to_owned(),
                is_match: *is_match,
                tx: Some(proto::TransactionContext {
                    tx_hash: (*tx).to_owned(),
                    log_index: i as u64,
                    ..Default::default()
                })
                .into(),
                ..Default::default()
            })
            .collect()
    }

    fn volume_flags(fills: &[(&str, &str, bool)]) -> Vec<bool> {
        let mut t = tape(fills);
        mark_volume(&mut t);
        t.iter().map(|t| t.is_volume).collect()
    }

    #[test]
    fn volume_counts_taker_fill_for_same_token_match() {
        // Two makers sell X to a taker buying X: only the taker fill is volume.
        assert_eq!(
            volume_flags(&[("t1", "X", false), ("t1", "X", false), ("t1", "X", true)]),
            [false, false, true]
        );
    }

    #[test]
    fn volume_counts_complement_maker_fills_in_mint_match() {
        // Taker buys X; one maker sells X, one buys the complement X' (mint).
        assert_eq!(
            volume_flags(&[("t1", "X'", false), ("t1", "X", false), ("t1", "X", true)]),
            [true, false, true]
        );
    }

    #[test]
    fn volume_counts_fills_without_taker_and_splits_by_tx() {
        // t1: a direct fillOrder (no taker fill). t2: a normal match.
        assert_eq!(
            volume_flags(&[("t1", "X", false), ("t2", "X", false), ("t2", "X", true)]),
            [true, false, true]
        );
        // Two matches in one transaction.
        assert_eq!(
            volume_flags(&[
                ("t1", "X", false),
                ("t1", "X", true),
                ("t1", "Y", false),
                ("t1", "Y", true)
            ]),
            [false, true, false, true]
        );
    }

    #[test]
    fn malformed_amount_is_skipped() {
        let mut f = fill(CTF_EXCHANGE_FORMATTED, 1);
        f.fee = "not a number".to_owned();
        assert!(to_trade(f, "ctf_v2".into()).is_none());
    }
}

/// Runs `map_trades` on real blocks. The fixtures are raw blocks, so the test builds
/// the two inputs itself from the block's exchange logs (all three exchanges share
/// the CTF Exchange V2 event signatures); the on-chain `tests/assertions.yaml` run
/// covers the real upstream modules.
#[cfg(test)]
mod fixture_tests {
    use super::*;
    use polymarket_substreams_common::abi::ctf_exchange::events::{OrderFilled, OrdersMatched};
    use polymarket_substreams_common::{
        bigint_to_string, bigint_to_u32, block_logs, format_address, with_lazy_block, CTF_EXCHANGE,
        EXCHANGE_V3, NEG_RISK_CTF,
    };

    struct Inputs {
        exchange: exchange::ExchangeEvents,
        neg_risk: neg_risk::TradingEvents,
        orders_matched: usize,
    }

    fn inputs(block_number: u64) -> Inputs {
        let path = format!(
            "{}/../testdata/polygon_{block_number}.binpb.b64",
            env!("CARGO_MANIFEST_DIR")
        );
        let block = polymarket_substreams_common::load_block_b64(&path).expect("fixture loads");
        with_lazy_block(&block, |blk| {
            let blk = block_logs(blk, |a| {
                a == CTF_EXCHANGE || a == NEG_RISK_CTF || a == EXCHANGE_V3
            })
            .expect("logs decode");
            let mut out = Inputs {
                exchange: Default::default(),
                neg_risk: Default::default(),
                orders_matched: 0,
            };
            for log in &blk.logs {
                if OrdersMatched::match_log(&log.log) {
                    out.orders_matched += 1;
                    continue;
                }
                let Ok(e) = OrderFilled::decode(&log.log) else {
                    continue;
                };
                let tx = exchange::TransactionContext {
                    tx_hash: format_address(log.tx_hash),
                    log_index: log.log.block_index as u64,
                    block_number: blk.number,
                    timestamp: blk.timestamp,
                };
                let fill = exchange::OrderFilled {
                    order_hash: e.order_hash.to_vec(),
                    maker: format_address(&e.maker),
                    taker: format_address(&e.taker),
                    side: bigint_to_u32(&e.side),
                    token_id: bigint_to_string(&e.token_id),
                    maker_amount_filled: bigint_to_string(&e.maker_amount_filled),
                    taker_amount_filled: bigint_to_string(&e.taker_amount_filled),
                    fee: bigint_to_string(&e.fee),
                    tx: tx.into(),
                    exchange: if log.log.address == CTF_EXCHANGE {
                        "ctf_v2".into()
                    } else {
                        "exchange_v3".into()
                    },
                    ..Default::default()
                };
                if log.log.address == NEG_RISK_CTF {
                    let tx = fill.tx.as_option().expect("tx");
                    out.neg_risk.order_filled.push(neg_risk::OrderFilled {
                        order_hash: fill.order_hash,
                        maker: fill.maker,
                        taker: fill.taker,
                        side: fill.side,
                        token_id: fill.token_id,
                        maker_amount_filled: fill.maker_amount_filled,
                        taker_amount_filled: fill.taker_amount_filled,
                        fee: fill.fee,
                        tx: neg_risk::TransactionContext {
                            tx_hash: tx.tx_hash.clone(),
                            log_index: tx.log_index,
                            block_number: tx.block_number,
                            timestamp: tx.timestamp,
                        }
                        .into(),
                        ..Default::default()
                    });
                } else {
                    out.exchange.order_filled.push(fill);
                }
            }
            Ok::<_, Error>(out)
        })
        .expect("fixture block encodes")
        .expect("inputs build")
    }

    fn count(trades: &proto::Trades, exchange: &str) -> usize {
        trades
            .trades
            .iter()
            .filter(|t| t.exchange == exchange)
            .count()
    }

    #[test]
    fn map_trades_real_block_96600000() {
        let inp = inputs(96600000);
        let fills = inp.exchange.order_filled.len() + inp.neg_risk.order_filled.len();
        let matched = inp.orders_matched;
        let out = substreams::testing::map!(map_trades(inp.exchange, inp.neg_risk))
            .expect("handler must not err");
        // One trade per OrderFilled, from both exchanges (36 CTF V2 + 14 Neg Risk).
        assert_eq!(out.trades.len(), fills);
        assert_eq!(
            (count(&out, "ctf_v2"), count(&out, "neg_risk_v2")),
            (36, 14)
        );
        // Exactly one is_match trade per OrdersMatched (9 + 6).
        assert_eq!(out.trades.iter().filter(|t| t.is_match).count(), matched);
        assert_eq!(matched, 15);
        // Block order.
        let idx: Vec<u64> = out
            .trades
            .iter()
            .map(|t| t.tx.as_option().expect("tx").log_index)
            .collect();
        assert!(idx.windows(2).all(|w| w[0] < w[1]));
        // Pinned against `substreams run polymarket-trades map_trades` on this block.
        let first = &out.trades[0];
        assert_eq!(first.exchange, FIRST_96600000.0);
        assert_eq!(first.maker, FIRST_96600000.1);
        assert_eq!(first.price, FIRST_96600000.2);
    }

    #[test]
    fn map_trades_real_block_96680009_includes_exchange_v3() {
        let inp = inputs(96680009);
        let matched = inp.orders_matched;
        let out = substreams::testing::map!(map_trades(inp.exchange, inp.neg_risk))
            .expect("handler must not err");
        assert_eq!(count(&out, "exchange_v3"), 4);
        assert_eq!(out.trades.iter().filter(|t| t.is_match).count(), matched);
        // The ExchangeV3 taker fill (see polymarket-exchange's fixture test): BUY
        // 106231679 shares for 952790 + 47210 fee = 1000000 collateral.
        let v3_match = out
            .trades
            .iter()
            .find(|t| t.exchange == "exchange_v3" && t.is_match)
            .expect("an ExchangeV3 match");
        assert_eq!(v3_match.shares, "106231679");
        assert_eq!(v3_match.collateral, "1000000");
        assert_eq!(v3_match.price, "0.009413");
    }

    /// Market stats over block 96600000, with in-memory stores standing in for
    /// `store_token_totals` / `store_last_price`.
    #[test]
    fn map_market_stats_real_block_96600000() {
        let inp = inputs(96600000);
        let trades = substreams::testing::map!(map_trades(inp.exchange, inp.neg_risk))
            .expect("handler must not err");
        let stats = crate::stats::tests::stats_after(&[&trades]);
        let mut tokens: Vec<&str> = trades.trades.iter().map(|t| t.token_id.as_str()).collect();
        tokens.sort_unstable();
        tokens.dedup();
        // One entry per token traded in the block.
        assert_eq!(stats.tokens.len(), tokens.len());
        assert_eq!(stats.tokens.len(), TOKENS_96600000);
        for s in &stats.tokens {
            let volume: BigInt = trades
                .trades
                .iter()
                .filter(|t| t.token_id == s.token_id && t.is_volume)
                .fold(BigInt::zero(), |acc, t| {
                    acc + BigInt::from_str(&t.collateral).expect("amount")
                });
            assert_eq!(s.volume_collateral, volume.to_string(), "{}", s.token_id);
            assert_eq!(s.block_volume_collateral, s.volume_collateral);
            assert_eq!(s.last_trade_block, 96600000);
        }
        // Every match counted once: is_volume trades are the 15 taker fills plus the
        // maker fills on a complement token.
        let volume_fills = trades.trades.iter().filter(|t| t.is_volume).count();
        assert_eq!(volume_fills, VOLUME_FILLS_96600000);
    }

    /// Candles over block 96600000: one per token with is_volume trades, and their
    /// volumes add up to the block's is_volume volume.
    #[test]
    fn map_candles_real_block_96600000() {
        let inp = inputs(96600000);
        let trades = substreams::testing::map!(map_trades(inp.exchange, inp.neg_risk))
            .expect("handler must not err");
        let candles = substreams::testing::map!(crate::candles::map_candles(trades.clone()))
            .expect("handler must not err");
        let mut tokens: Vec<&str> = trades
            .trades
            .iter()
            .filter(|t| t.is_volume)
            .map(|t| t.token_id.as_str())
            .collect();
        tokens.sort_unstable();
        tokens.dedup();
        assert_eq!(candles.candles.len(), tokens.len());
        assert_eq!(candles.candles.len(), CANDLES_96600000);
        let total = |f: fn(&proto::Candle) -> &str| {
            candles.candles.iter().fold(BigInt::zero(), |acc, c| {
                acc + BigInt::from_str(f(c)).expect("amount")
            })
        };
        let volume: BigInt = trades
            .trades
            .iter()
            .filter(|t| t.is_volume)
            .fold(BigInt::zero(), |acc, t| {
                acc + BigInt::from_str(&t.collateral).expect("amount")
            });
        assert_eq!(total(|c| &c.volume_collateral), volume);
        let n: u64 = candles.candles.iter().map(|c| c.trades).sum();
        assert_eq!(n, VOLUME_FILLS_96600000 as u64);
        for c in &candles.candles {
            assert_eq!(c.bucket_start % 60, 0);
            assert!(c.low <= c.open && c.open <= c.high, "{}", c.token_id);
            assert!(c.low <= c.close && c.close <= c.high, "{}", c.token_id);
        }
        let first = &candles.candles[0];
        assert_eq!(
            (first.token_id.as_str(), first.open.as_str(), first.bucket_start),
            FIRST_CANDLE_96600000
        );
    }

    /// Order fills over block 96600000: one entry per order hash, every fill
    /// counted once, amounts equal to the sum of that order's trades.
    #[test]
    fn map_order_fills_real_block_96600000() {
        let inp = inputs(96600000);
        let trades = substreams::testing::map!(map_trades(inp.exchange, inp.neg_risk))
            .expect("handler must not err");
        let fills = substreams::testing::map!(crate::orders::map_order_fills(trades.clone()))
            .expect("handler must not err");
        let mut hashes: Vec<&[u8]> = trades.trades.iter().map(|t| t.order_hash.as_slice()).collect();
        hashes.sort_unstable();
        hashes.dedup();
        assert_eq!(fills.orders.len(), hashes.len());
        assert_eq!(fills.orders.len(), ORDERS_96600000);
        let n: u64 = fills.orders.iter().map(|o| o.fills).sum();
        assert_eq!(n, trades.trades.len() as u64);
        for o in &fills.orders {
            assert_eq!(o.block_number, 96600000);
            let shares = trades
                .trades
                .iter()
                .filter(|t| format_address(&t.order_hash) == o.order_hash)
                .fold(BigInt::zero(), |acc, t| {
                    acc + BigInt::from_str(&t.shares).expect("amount")
                });
            assert_eq!(o.shares, shares.to_string(), "{}", o.order_hash);
        }
        assert_eq!(fills.orders.iter().filter(|o| o.fills > 1).count(), 1);
    }

    /// Protocol V2 module `ConditionResolved` logs of a block, as
    /// polymarket-resolution's `map_resolution_events` emits them (decoded by hand
    /// here: that package is a cdylib, so it can't be a dev-dependency).
    fn module_resolutions(block_number: u64) -> pb::polymarket::resolution::v1::ResolutionEvents {
        use pb::polymarket::resolution::v1 as res;
        const BINARY: [u8; 20] = hex_literal::hex!("1000008dd9001b968442c1000017eae6e0da00ba");
        const NEG_RISK: [u8; 20] = hex_literal::hex!("200000900045e3b6259600682756002200028933");
        const TOPIC0: [u8; 32] = hex_literal::hex!(
            "e560c1a2847d4cf2e66de11fe592605556989449f38cc3ab96e2e287daa68caa"
        );
        let path = format!(
            "{}/../testdata/polygon_{block_number}.binpb.b64",
            env!("CARGO_MANIFEST_DIR")
        );
        let block = polymarket_substreams_common::load_block_b64(&path).expect("fixture loads");
        with_lazy_block(&block, |blk| {
            let blk = block_logs(blk, |a| a == BINARY || a == NEG_RISK).expect("logs decode");
            let mut out = res::ModuleEvents::default();
            for log in &blk.logs {
                let topics: Vec<&[u8]> = log.log.topics.iter().copied().collect();
                if topics.len() != 2 || topics[0] != TOPIC0 {
                    continue;
                }
                // data: offset, length, then one 32-byte word per payout.
                let data = log.log.data;
                let n = BigInt::from_unsigned_bytes_be(&data[32..64]).to_u64() as usize;
                let payouts = (0..n)
                    .map(|i| {
                        BigInt::from_unsigned_bytes_be(&data[64 + 32 * i..96 + 32 * i]).to_string()
                    })
                    .collect();
                out.condition_resolved.push(res::ModuleConditionResolved {
                    condition_id: format_address(&topics[1][..31]),
                    payouts,
                    module: if log.log.address == BINARY { "binary" } else { "neg_risk" }.into(),
                    tx: Some(res::TransactionContext {
                        tx_hash: format_address(log.tx_hash),
                        log_index: log.log.block_index as u64,
                        block_number: blk.number,
                        timestamp: blk.timestamp,
                    })
                    .into(),
                });
            }
            res::ResolutionEvents {
                modules: Some(out).into(),
                ..Default::default()
            }
        })
        .expect("fixture decodes")
    }

    /// db_out / db_out_clickhouse over block 96600000 (trades, market stats,
    /// candles, order fills) and 96681462 (Protocol V2 module resolutions).
    #[test]
    fn db_out_real_blocks() {
        use crate::db::tests::{clickhouse, postgres, rows};
        use crate::db::{CANDLES, MARKET_STATS, ORDER_FILLS, RESOLUTIONS, TRADES};
        use substreams_database_change::pb::sf::substreams::sink::database::v1::table_change::Operation;

        let inp = inputs(96600000);
        let trades = substreams::testing::map!(map_trades(inp.exchange, inp.neg_risk))
            .expect("handler must not err");
        let stats = crate::stats::tests::stats_after(&[&trades]);
        let candles = substreams::testing::map!(crate::candles::map_candles(trades.clone()))
            .expect("handler must not err");
        let fills = substreams::testing::map!(crate::orders::map_order_fills(trades.clone()))
            .expect("handler must not err");
        let resolutions = module_resolutions(96681462);
        let no_res = Default::default();

        let pg = postgres(&trades, &stats, &candles, &fills, &no_res);
        assert_eq!(rows(&pg, TRADES), (50, Some(Operation::Create)));
        assert_eq!(rows(&pg, MARKET_STATS), (21, Some(Operation::Upsert)));
        assert_eq!(rows(&pg, CANDLES), (21, Some(Operation::Upsert)));
        assert_eq!(rows(&pg, ORDER_FILLS), (ORDERS_96600000, Some(Operation::Upsert)));
        assert_eq!(rows(&pg, RESOLUTIONS).0, 0);
        let first = pg
            .table_changes
            .iter()
            .find(|c| c.table == TRADES)
            .expect("a trade row");
        let price = first.fields.iter().find(|f| f.name == "price").expect("price");
        assert_eq!(price.value, FIRST_96600000.2);

        let pg = postgres(
            &proto::Trades::default(),
            &Default::default(),
            &Default::default(),
            &Default::default(),
            &resolutions,
        );
        assert_eq!(rows(&pg, RESOLUTIONS), (41, Some(Operation::Create)));
        let r = pg
            .table_changes
            .iter()
            .find(|c| c.table == RESOLUTIONS)
            .expect("a resolution row");
        let field = |name: &str| {
            r.fields
                .iter()
                .find(|f| f.name == name)
                .map(|f| f.value.clone())
                .expect("field")
        };
        assert_eq!(field("source"), "binary");
        assert_eq!(field("payouts"), "0,1000000");
        assert!(field("resolved_id").starts_with("0x01c4dffb"));

        // ClickHouse: the same rows, all plain inserts.
        let ch = clickhouse(&trades, &stats, &candles, &fills, &resolutions);
        for (table, n) in [
            (TRADES, 50),
            (MARKET_STATS, 21),
            (CANDLES, 21),
            (ORDER_FILLS, ORDERS_96600000),
            (RESOLUTIONS, 41),
        ] {
            assert_eq!(rows(&ch, table), (n, Some(Operation::Create)), "{table}");
        }
    }

    /// (exchange, maker, price) of the first trade in block 96600000.
    const FIRST_96600000: (&str, &str, &str) = (
        "ctf_v2",
        "0x45f82472a88f759f8a98174d3cdd3db4a9ee6d79",
        "0.400000",
    );
    const TOKENS_96600000: usize = 21;
    const VOLUME_FILLS_96600000: usize = 43;
    const CANDLES_96600000: usize = 21;
    const ORDERS_96600000: usize = 49;
    /// (token_id, open, bucket_start) of the first candle in block 96600000.
    const FIRST_CANDLE_96600000: (&str, &str, u64) = (
        "104682004226386236263369368652385253547958068860089723081382793939737139300525",
        "0.166720",
        1793605440,
    );
}
