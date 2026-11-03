//! SQL sink output: `db_out` for PostgreSQL (schema.sql) and `db_out_clickhouse` for
//! ClickHouse (clickhouse-schema.sql), both `DatabaseChanges` for `substreams sink`.
//!
//! PostgreSQL merges candle and order-fill fragments in place with delta upserts
//! (`add`, `max`, `min`, `set_if_null`). The ClickHouse sink has no delta updates,
//! so `db_out_clickhouse` inserts every fragment as its own row and the table
//! engines merge them.

use std::collections::HashMap;
use std::str::FromStr;

use substreams::errors::Error;
use substreams::scalar::{BigDecimal, BigInt};
use substreams_database_change::pb::sf::substreams::sink::database::v1::DatabaseChanges;
use substreams_database_change::tables::Tables;

use crate::pb::polymarket::resolution::v1 as resolution;
use crate::pb::polymarket::trades::v1 as proto;

pub(crate) const TRADES: &str = "trades";
pub(crate) const MARKET_STATS: &str = "market_stats";
pub(crate) const CANDLES: &str = "candles_1m";
pub(crate) const ORDER_FILLS: &str = "order_fills";
pub(crate) const RESOLUTIONS: &str = "resolutions";
pub(crate) const ORDERS: &str = "orders";

#[derive(Clone, Copy, PartialEq)]
enum Dialect {
    Postgres,
    ClickHouse,
}

#[substreams::handlers::map]
pub fn db_out(
    trades: proto::Trades,
    stats: proto::MarketStats,
    candles: proto::Candles,
    order_fills: proto::OrderFills,
    resolutions: resolution::ResolutionEvents,
    orders: proto::ObservedOrders,
) -> Result<DatabaseChanges, Error> {
    substreams::skip_empty_output();
    Ok(build(
        Dialect::Postgres,
        &trades,
        &stats,
        &candles,
        &order_fills,
        &resolutions,
        &orders,
    ))
}

#[substreams::handlers::map]
pub fn db_out_clickhouse(
    trades: proto::Trades,
    stats: proto::MarketStats,
    candles: proto::Candles,
    order_fills: proto::OrderFills,
    resolutions: resolution::ResolutionEvents,
    orders: proto::ObservedOrders,
) -> Result<DatabaseChanges, Error> {
    substreams::skip_empty_output();
    Ok(build(
        Dialect::ClickHouse,
        &trades,
        &stats,
        &candles,
        &order_fills,
        &resolutions,
        &orders,
    ))
}

/// Parses a decimal integer amount; malformed input counts as zero rather than
/// reaching the crate's delta ops, which panic on non-numeric strings.
fn int(value: &str) -> BigInt {
    BigInt::from_str(value).unwrap_or_else(|_| BigInt::zero())
}

fn decimal(value: &str) -> Option<BigDecimal> {
    BigDecimal::from_str(value).ok()
}

fn build(
    dialect: Dialect,
    trades: &proto::Trades,
    stats: &proto::MarketStats,
    candles: &proto::Candles,
    order_fills: &proto::OrderFills,
    resolutions: &resolution::ResolutionEvents,
    orders: &proto::ObservedOrders,
) -> DatabaseChanges {
    let mut tables = Tables::new();
    // Block timestamp of this block, for tables whose input carries none.
    let mut block_timestamp = 0;

    for t in &trades.trades {
        let Some(tx) = t.tx.as_option() else {
            continue;
        };
        block_timestamp = tx.timestamp;
        tables
            .create_row(TRADES, format!("{}-{}", tx.tx_hash, tx.log_index))
            .set("token_id", &t.token_id)
            .set("side", t.side)
            .set("shares", int(&t.shares))
            .set("collateral", int(&t.collateral))
            .set("price", &t.price)
            .set("fee", int(&t.fee))
            .set("maker", &t.maker)
            .set("taker", &t.taker)
            .set("order_hash", format!("0x{}", hex::encode(&t.order_hash)))
            .set("exchange", &t.exchange)
            .set("is_match", t.is_match)
            .set("is_volume", t.is_volume)
            .set("tx_hash", &tx.tx_hash)
            .set("log_index", tx.log_index)
            .set("block_number", tx.block_number)
            .set("block_timestamp", tx.timestamp);
    }

    for s in &stats.tokens {
        let row = match dialect {
            // One row per token, replaced by each block that trades it.
            Dialect::Postgres => tables.upsert_row(MARKET_STATS, &s.token_id),
            // ReplacingMergeTree(last_trade_block) keeps the latest row per token.
            Dialect::ClickHouse => tables.create_row(
                MARKET_STATS,
                [
                    ("token_id", s.token_id.clone()),
                    ("last_trade_block", s.last_trade_block.to_string()),
                ],
            ),
        };
        row.set("trades", s.trades)
            .set("buys", s.buys)
            .set("sells", s.sells)
            .set("volume_collateral", int(&s.volume_collateral))
            .set("volume_shares", int(&s.volume_shares))
            .set("fees", int(&s.fees))
            .set("last_price", &s.last_price)
            .set("block_vwap", &s.block_vwap)
            .set("block_volume_collateral", int(&s.block_volume_collateral))
            .set("block_volume_shares", int(&s.block_volume_shares))
            .set("last_trade_timestamp", s.last_trade_timestamp);
        if dialect == Dialect::Postgres {
            row.set("last_trade_block", s.last_trade_block);
        }
    }

    for c in &candles.candles {
        let (Some(open), Some(high), Some(low), Some(close)) = (
            decimal(&c.open),
            decimal(&c.high),
            decimal(&c.low),
            decimal(&c.close),
        ) else {
            continue;
        };
        match dialect {
            // Blocks arrive in order, so the first fragment's open is the bucket's
            // open (set_if_null) and the last fragment's close its close (set).
            Dialect::Postgres => {
                tables
                    .upsert_row(
                        CANDLES,
                        [
                            ("token_id", c.token_id.clone()),
                            ("bucket_start", c.bucket_start.to_string()),
                        ],
                    )
                    .set_if_null("open", &open)
                    .max("high", &high)
                    .min("low", &low)
                    .set("close", &close)
                    .add("volume_shares", int(&c.volume_shares))
                    .add("volume_collateral", int(&c.volume_collateral))
                    .add("trades", c.trades as i64)
                    .set_if_null("first_block", c.block_number)
                    .set("last_block", c.block_number);
            }
            Dialect::ClickHouse => {
                tables
                    .create_row(
                        CANDLES,
                        [
                            ("token_id", c.token_id.clone()),
                            ("bucket_start", c.bucket_start.to_string()),
                            ("block_number", c.block_number.to_string()),
                        ],
                    )
                    .set("open", &open)
                    .set("high", &high)
                    .set("low", &low)
                    .set("close", &close)
                    .set("volume_shares", int(&c.volume_shares))
                    .set("volume_collateral", int(&c.volume_collateral))
                    .set("trades", c.trades)
                    .set("open_log_index", c.open_log_index)
                    .set("close_log_index", c.close_log_index);
            }
        }
    }

    for o in &order_fills.orders {
        match dialect {
            Dialect::Postgres => {
                tables
                    .upsert_row(ORDER_FILLS, &o.order_hash)
                    .set_if_null("owner", &o.owner)
                    .set_if_null("token_id", &o.token_id)
                    .set_if_null("side", o.side)
                    .set_if_null("exchange", &o.exchange)
                    .add("filled_shares", int(&o.shares))
                    .add("filled_collateral", int(&o.collateral))
                    .add("fees", int(&o.fees))
                    .add("fills", o.fills as i64)
                    .set_if_null("first_fill_block", o.block_number)
                    .set("last_fill_block", o.block_number)
                    .set("last_fill_timestamp", o.timestamp);
            }
            Dialect::ClickHouse => {
                tables
                    .create_row(
                        ORDER_FILLS,
                        [
                            ("order_hash", o.order_hash.clone()),
                            ("block_number", o.block_number.to_string()),
                        ],
                    )
                    .set("owner", &o.owner)
                    .set("token_id", &o.token_id)
                    .set("side", o.side)
                    .set("exchange", &o.exchange)
                    .set("shares", int(&o.shares))
                    .set("collateral", int(&o.collateral))
                    .set("fees", int(&o.fees))
                    .set("fills", o.fills)
                    .set("block_timestamp", o.timestamp);
            }
        }
    }

    push_resolutions(&mut tables, resolutions, block_timestamp);
    push_orders(&mut tables, dialect, orders);
    tables.to_database_changes()
}

/// The full signed orders behind the fills. PostgreSQL keeps one row per order and
/// sums each settlement's `fill_amount` into `filled`, so `maker_amount - filled`
/// is what remains. ClickHouse gets one row per settlement of an order.
fn push_orders(tables: &mut Tables, dialect: Dialect, orders: &proto::ObservedOrders) {
    match dialect {
        Dialect::Postgres => {
            // An order can settle twice in one block. Upserting the same key twice
            // in one block would overwrite the first delta, so sum per order first.
            let mut per_order: Vec<(
                &proto::ObservedOrder,
                BigInt,
                BigInt,
                i64,
                &proto::TransactionContext,
            )> = Vec::new();
            let mut index: HashMap<&str, usize> = HashMap::new();
            for o in &orders.orders {
                let Some(tx) = o.tx.as_option() else {
                    continue;
                };
                let (fill, fee) = (int(&o.fill_amount), int(&o.fee));
                match index.get(o.order_hash.as_str()) {
                    Some(&i) => {
                        let e = &mut per_order[i];
                        e.1 = e.1.clone() + fill;
                        e.2 = e.2.clone() + fee;
                        e.3 += 1;
                        e.4 = tx;
                    }
                    None => {
                        index.insert(&o.order_hash, per_order.len());
                        per_order.push((o, fill, fee, 1, tx));
                    }
                }
            }
            for (o, filled, fees, settlements, last_tx) in per_order {
                tables
                    .upsert_row(ORDERS, &o.order_hash)
                    .set_if_null("exchange", &o.exchange)
                    .set_if_null("maker", &o.maker)
                    .set_if_null("signer", &o.signer)
                    .set_if_null("token_id", &o.token_id)
                    .set_if_null("side", o.side)
                    .set_if_null("maker_amount", int(&o.maker_amount))
                    .set_if_null("taker_amount", int(&o.taker_amount))
                    .set_if_null("limit_price", &o.limit_price)
                    .set_if_null("size_shares", int(&o.size_shares))
                    .set_if_null("signature_type", o.signature_type)
                    .set_if_null("created_at_ms", o.created_at_ms)
                    .set_if_null("metadata", &o.metadata)
                    .set_if_null("builder", &o.builder)
                    .add("filled", filled)
                    .add("fees", fees)
                    .add("settlements", settlements)
                    .set_if_null("first_block", last_tx.block_number)
                    .set("last_block", last_tx.block_number)
                    .set("last_timestamp", last_tx.timestamp);
            }
        }
        Dialect::ClickHouse => {
            for o in &orders.orders {
                let Some(tx) = o.tx.as_option() else {
                    continue;
                };
                tables
                    .create_row(
                        ORDERS,
                        [
                            ("order_hash", o.order_hash.clone()),
                            ("block_number", tx.block_number.to_string()),
                            ("log_index", tx.log_index.to_string()),
                        ],
                    )
                    .set("exchange", &o.exchange)
                    .set("maker", &o.maker)
                    .set("signer", &o.signer)
                    .set("token_id", &o.token_id)
                    .set("side", o.side)
                    .set("maker_amount", int(&o.maker_amount))
                    .set("taker_amount", int(&o.taker_amount))
                    .set("limit_price", &o.limit_price)
                    .set("size_shares", int(&o.size_shares))
                    .set("signature_type", o.signature_type)
                    .set("created_at_ms", o.created_at_ms)
                    .set("metadata", &o.metadata)
                    .set("builder", &o.builder)
                    .set("is_taker", o.is_taker)
                    .set("fill_amount", int(&o.fill_amount))
                    .set("fee", int(&o.fee))
                    .set("tx_hash", &tx.tx_hash)
                    .set("block_timestamp", tx.timestamp);
            }
        }
    }
}

/// One `resolutions` row per settled outcome: UMA adapter `QuestionResolved` and
/// `QuestionEmergencyResolved` (keyed by question id), and Protocol V2 module
/// `ConditionResolved` (keyed by condition id).
fn push_resolutions(
    tables: &mut Tables,
    events: &resolution::ResolutionEvents,
    fallback_timestamp: u64,
) {
    let mut push = |source: &str,
                    id: String,
                    settled_price: &str,
                    payouts: &[String],
                    tx: Option<&resolution::TransactionContext>| {
        let Some(tx) = tx else {
            return;
        };
        let timestamp = if tx.timestamp != 0 {
            tx.timestamp
        } else {
            fallback_timestamp
        };
        tables
            .create_row(RESOLUTIONS, format!("{}-{}", tx.tx_hash, tx.log_index))
            .set("source", source)
            .set("resolved_id", id)
            .set("settled_price", settled_price)
            .set("payouts", payouts.join(","))
            .set("tx_hash", &tx.tx_hash)
            .set("log_index", tx.log_index)
            .set("block_number", tx.block_number)
            .set("block_timestamp", timestamp);
    };
    if let Some(adapter) = events.adapter.as_option() {
        for r in &adapter.question_resolved {
            push(
                "uma_adapter",
                format!("0x{}", hex::encode(&r.question_id)),
                &r.settled_price,
                &r.payouts,
                r.tx.as_option(),
            );
        }
        for r in &adapter.question_emergency_resolved {
            push(
                "uma_adapter_emergency",
                format!("0x{}", hex::encode(&r.question_id)),
                "",
                &r.payouts,
                r.tx.as_option(),
            );
        }
    }
    if let Some(modules) = events.modules.as_option() {
        for r in &modules.condition_resolved {
            push(
                &r.module,
                r.condition_id.clone(),
                "",
                &r.payouts,
                r.tx.as_option(),
            );
        }
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use substreams_database_change::pb::sf::substreams::sink::database::v1::table_change::Operation;

    pub(crate) fn postgres(
        trades: &proto::Trades,
        stats: &proto::MarketStats,
        candles: &proto::Candles,
        order_fills: &proto::OrderFills,
        resolutions: &resolution::ResolutionEvents,
        orders: &proto::ObservedOrders,
    ) -> DatabaseChanges {
        build(
            Dialect::Postgres,
            trades,
            stats,
            candles,
            order_fills,
            resolutions,
            orders,
        )
    }

    pub(crate) fn clickhouse(
        trades: &proto::Trades,
        stats: &proto::MarketStats,
        candles: &proto::Candles,
        order_fills: &proto::OrderFills,
        resolutions: &resolution::ResolutionEvents,
        orders: &proto::ObservedOrders,
    ) -> DatabaseChanges {
        build(
            Dialect::ClickHouse,
            trades,
            stats,
            candles,
            order_fills,
            resolutions,
            orders,
        )
    }

    /// Rows per table, and the operation they all share.
    pub(crate) fn rows(changes: &DatabaseChanges, table: &str) -> (usize, Option<Operation>) {
        let rows: Vec<_> = changes
            .table_changes
            .iter()
            .filter(|c| c.table == table)
            .collect();
        let op = rows.first().and_then(|c| c.operation.as_known());
        assert!(rows.iter().all(|c| c.operation.as_known() == op));
        (rows.len(), op)
    }

    #[test]
    fn malformed_amounts_do_not_panic() {
        let trades = proto::Trades {
            trades: vec![proto::Trade {
                shares: "x".into(),
                collateral: "".into(),
                fee: "1e".into(),
                tx: Some(proto::TransactionContext::default()).into(),
                ..Default::default()
            }],
        };
        let orders = proto::OrderFills {
            orders: vec![proto::OrderFill {
                order_hash: "0x01".into(),
                shares: "bad".into(),
                ..Default::default()
            }],
        };
        let candles = proto::Candles {
            candles: vec![proto::Candle {
                open: "not a price".into(),
                ..Default::default()
            }],
        };
        let d = postgres(
            &trades,
            &Default::default(),
            &candles,
            &orders,
            &Default::default(),
            &proto::ObservedOrders {
                orders: vec![proto::ObservedOrder {
                    order_hash: "0x02".into(),
                    maker_amount: "".into(),
                    fill_amount: "?".into(),
                    tx: Some(proto::TransactionContext::default()).into(),
                    ..Default::default()
                }],
            },
        );
        assert_eq!(rows(&d, TRADES).0, 1);
        assert_eq!(rows(&d, ORDERS).0, 1);
        assert_eq!(rows(&d, ORDER_FILLS).0, 1);
        // A candle with an unparseable price is skipped.
        assert_eq!(rows(&d, CANDLES).0, 0);
    }

    #[test]
    fn order_settled_twice_in_a_block_is_one_upsert_with_both_fills() {
        let settle = |fill: &str, log_index: u64| proto::ObservedOrder {
            order_hash: "0xaa".into(),
            maker_amount: "100".into(),
            fill_amount: fill.into(),
            fee: "1".into(),
            tx: Some(proto::TransactionContext {
                log_index,
                block_number: 7,
                ..Default::default()
            })
            .into(),
            ..Default::default()
        };
        let orders = proto::ObservedOrders {
            orders: vec![settle("30", 1), settle("50", 9)],
        };
        let none = Default::default();
        let pg = postgres(
            &none,
            &none_stats(),
            &none_candles(),
            &none_fills(),
            &Default::default(),
            &orders,
        );
        assert_eq!(rows(&pg, ORDERS), (1, Some(Operation::Upsert)));
        let row = pg
            .table_changes
            .iter()
            .find(|c| c.table == ORDERS)
            .expect("row");
        let field = |n: &str| {
            row.fields
                .iter()
                .find(|f| f.name == n)
                .map(|f| f.value.clone())
        };
        assert_eq!(field("filled").as_deref(), Some("80"));
        assert_eq!(field("fees").as_deref(), Some("2"));
        assert_eq!(field("settlements").as_deref(), Some("2"));
        assert_eq!(field("maker_amount").as_deref(), Some("100"));

        let ch = clickhouse(
            &none,
            &none_stats(),
            &none_candles(),
            &none_fills(),
            &Default::default(),
            &orders,
        );
        assert_eq!(rows(&ch, ORDERS), (2, Some(Operation::Create)));
    }

    fn none_stats() -> proto::MarketStats {
        Default::default()
    }
    fn none_candles() -> proto::Candles {
        Default::default()
    }
    fn none_fills() -> proto::OrderFills {
        Default::default()
    }
}
