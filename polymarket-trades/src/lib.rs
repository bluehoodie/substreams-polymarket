//! Polymarket trade tape: one `Trade` per `OrderFilled`, read from the outputs of the
//! sibling exchange packages (no block decoding here).

#[allow(dead_code, clippy::all)]
pub mod pb;

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
const SIDE_BUY: u32 = 0;

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
    Ok(proto::Trades { trades })
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
fn format_price(collateral: &BigInt, shares: &BigInt) -> String {
    if shares.is_zero() {
        return "0".to_owned();
    }
    let scaled = (collateral.clone() * BigInt::from(PRICE_SCALE)) / shares.clone();
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

    /// (exchange, maker, price) of the first trade in block 96600000.
    const FIRST_96600000: (&str, &str, &str) = (
        "ctf_v2",
        "0x45f82472a88f759f8a98174d3cdd3db4a9ee6d79",
        "0.400000",
    );
}
