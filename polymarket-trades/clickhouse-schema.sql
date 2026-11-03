-- ClickHouse schema for `db_out_clickhouse` (substreams sink clickhouse). The
-- ClickHouse sink only inserts, so candle and order fill fragments land as rows
-- (one per token/order per block) and are merged here: candles by an
-- AggregatingMergeTree fed from a materialized view, order fills by a view.
-- Amounts are raw 6-decimal integers, prices have 6 decimals, block_timestamp is
-- unix seconds.

CREATE TABLE IF NOT EXISTS trades (
    id               String,                    -- <tx_hash>-<log_index>
    token_id         String,
    side             UInt8,                     -- 0 = BUY, 1 = SELL
    shares           UInt256,
    collateral       UInt256,                   -- fee included
    price            Decimal(20, 6),
    fee              UInt256,
    maker            String,
    taker            String,
    order_hash       String,
    exchange         LowCardinality(String),    -- ctf_v2 | neg_risk_v2 | exchange_v3
    is_match         Bool,
    is_volume        Bool,
    tx_hash          String,
    log_index        UInt64,
    block_number     UInt64,
    block_timestamp  UInt64
) ENGINE = ReplacingMergeTree
PARTITION BY toYYYYMM(toDateTime(block_timestamp))
ORDER BY (token_id, block_timestamp, id);

-- Latest running summary per token (the row from the highest last_trade_block wins).
CREATE TABLE IF NOT EXISTS market_stats (
    token_id                 String,
    trades                   UInt64,
    buys                     UInt64,
    sells                    UInt64,
    volume_collateral        UInt256,
    volume_shares            UInt256,
    fees                     UInt256,
    last_price               Nullable(Decimal(20, 6)),
    block_vwap               Nullable(Decimal(20, 6)),
    block_volume_collateral  UInt256,
    block_volume_shares      UInt256,
    last_trade_block         UInt64,
    last_trade_timestamp     UInt64
) ENGINE = ReplacingMergeTree(last_trade_block)
ORDER BY token_id;

-- Per-block candle fragments, as the sink inserts them.
CREATE TABLE IF NOT EXISTS candles_1m (
    token_id           String,
    bucket_start       UInt64,                  -- unix seconds, multiple of 60
    block_number       UInt64,
    open               Decimal(20, 6),
    high               Decimal(20, 6),
    low                Decimal(20, 6),
    close              Decimal(20, 6),
    volume_shares      UInt256,
    volume_collateral  UInt256,
    trades             UInt64,
    open_log_index     UInt64,
    close_log_index    UInt64
) ENGINE = ReplacingMergeTree
PARTITION BY toYYYYMM(toDateTime(bucket_start))
ORDER BY (token_id, bucket_start, block_number);

-- 1-minute candles merged from the fragments: open/close from the lowest/highest
-- (block_number, log_index), high max, low min, volumes summed.
CREATE TABLE IF NOT EXISTS candles_1m_agg (
    token_id           String,
    bucket_start       UInt64,
    open               AggregateFunction(argMin, Decimal(20, 6), Tuple(UInt64, UInt64)),
    high               SimpleAggregateFunction(max, Decimal(20, 6)),
    low                SimpleAggregateFunction(min, Decimal(20, 6)),
    close              AggregateFunction(argMax, Decimal(20, 6), Tuple(UInt64, UInt64)),
    volume_shares      SimpleAggregateFunction(sum, UInt256),
    volume_collateral  SimpleAggregateFunction(sum, UInt256),
    trades             SimpleAggregateFunction(sum, UInt64)
) ENGINE = AggregatingMergeTree
PARTITION BY toYYYYMM(toDateTime(bucket_start))
ORDER BY (token_id, bucket_start);

CREATE MATERIALIZED VIEW IF NOT EXISTS candles_1m_mv TO candles_1m_agg AS
SELECT token_id,
       bucket_start,
       argMinState(open, (block_number, open_log_index)) AS open,
       max(high) AS high,
       min(low) AS low,
       argMaxState(close, (block_number, close_log_index)) AS close,
       sum(volume_shares) AS volume_shares,
       sum(volume_collateral) AS volume_collateral,
       sum(trades) AS trades
FROM candles_1m
GROUP BY token_id, bucket_start;

-- Finished 1-minute candles, and 5m / 1h / 1d rollups of them.
CREATE VIEW IF NOT EXISTS candles_1m_merged AS
SELECT token_id,
       bucket_start,
       argMinMerge(open) AS open,
       max(high) AS high,
       min(low) AS low,
       argMaxMerge(close) AS close,
       sum(volume_shares) AS volume_shares,
       sum(volume_collateral) AS volume_collateral,
       sum(trades) AS trades
FROM candles_1m_agg
GROUP BY token_id, bucket_start;

CREATE VIEW IF NOT EXISTS candles_5m AS
SELECT token_id,
       intDiv(bucket_start, 300) * 300 AS bucket,
       argMin(open, bucket_start) AS open,
       max(high) AS high,
       min(low) AS low,
       argMax(close, bucket_start) AS close,
       sum(volume_shares) AS volume_shares,
       sum(volume_collateral) AS volume_collateral,
       sum(trades) AS trades
FROM candles_1m_merged
GROUP BY token_id, bucket;

CREATE VIEW IF NOT EXISTS candles_1h AS
SELECT token_id,
       intDiv(bucket_start, 3600) * 3600 AS bucket,
       argMin(open, bucket_start) AS open,
       max(high) AS high,
       min(low) AS low,
       argMax(close, bucket_start) AS close,
       sum(volume_shares) AS volume_shares,
       sum(volume_collateral) AS volume_collateral,
       sum(trades) AS trades
FROM candles_1m_merged
GROUP BY token_id, bucket;

CREATE VIEW IF NOT EXISTS candles_1d AS
SELECT token_id,
       intDiv(bucket_start, 86400) * 86400 AS bucket,
       argMin(open, bucket_start) AS open,
       max(high) AS high,
       min(low) AS low,
       argMax(close, bucket_start) AS close,
       sum(volume_shares) AS volume_shares,
       sum(volume_collateral) AS volume_collateral,
       sum(trades) AS trades
FROM candles_1m_merged
GROUP BY token_id, bucket;

-- Per-block order fill fragments, as the sink inserts them, and the running total
-- per order.
CREATE TABLE IF NOT EXISTS order_fills (
    order_hash       String,
    block_number     UInt64,
    owner            String,
    token_id         String,
    side             UInt8,
    exchange         LowCardinality(String),
    shares           UInt256,
    collateral       UInt256,
    fees             UInt256,
    fills            UInt64,
    block_timestamp  UInt64
) ENGINE = ReplacingMergeTree
PARTITION BY toYYYYMM(toDateTime(block_timestamp))
ORDER BY (token_id, block_timestamp, order_hash, block_number);

CREATE VIEW IF NOT EXISTS order_fills_totals AS
SELECT order_hash,
       any(owner) AS owner,
       any(token_id) AS token_id,
       any(side) AS side,
       any(exchange) AS exchange,
       sum(shares) AS filled_shares,
       sum(collateral) AS filled_collateral,
       sum(fees) AS fees,
       sum(fills) AS fills,
       min(block_number) AS first_fill_block,
       max(block_number) AS last_fill_block,
       max(block_timestamp) AS last_fill_timestamp
FROM order_fills
GROUP BY order_hash;

-- The full signed orders behind the fills (map_orders), one row per settlement of
-- an order, and the order book view with what each order has left.
CREATE TABLE IF NOT EXISTS orders (
    order_hash       String,
    block_number     UInt64,
    log_index        UInt64,
    exchange         LowCardinality(String),
    maker            String,
    signer           String,
    token_id         String,
    side             UInt8,
    maker_amount     UInt256,
    taker_amount     UInt256,
    limit_price      String,
    size_shares      UInt256,
    signature_type   UInt8,
    created_at_ms    UInt64,
    metadata         String,
    builder          String,
    is_taker         Bool,
    fill_amount      UInt256,
    fee              UInt256,
    tx_hash          String,
    block_timestamp  UInt64
) ENGINE = ReplacingMergeTree
PARTITION BY toYYYYMM(toDateTime(block_timestamp))
ORDER BY (token_id, block_timestamp, order_hash, block_number, log_index);

CREATE VIEW IF NOT EXISTS order_book AS
SELECT order_hash,
       any(exchange) AS exchange,
       any(maker) AS maker,
       any(signer) AS signer,
       any(token_id) AS token_id,
       any(side) AS side,
       any(maker_amount) AS maker_amount,
       any(taker_amount) AS taker_amount,
       any(limit_price) AS limit_price,
       any(size_shares) AS size_shares,
       any(created_at_ms) AS created_at_ms,
       sum(fill_amount) AS filled,
       sum(fee) AS fees,
       count() AS settlements,
       min(block_number) AS first_block,
       max(block_number) AS last_block,
       if(filled >= maker_amount, toUInt256(0), maker_amount - filled) AS remaining,
       if(side = 0, intDiv(remaining * taker_amount, maker_amount), remaining) AS remaining_shares
FROM orders FINAL
GROUP BY order_hash;

-- Settled outcomes: UMA CTF adapter (resolved_id = question id) and Protocol V2
-- module ConditionResolved (resolved_id = condition id).
CREATE TABLE IF NOT EXISTS resolutions (
    id               String,                    -- <tx_hash>-<log_index>
    source           LowCardinality(String),    -- uma_adapter | uma_adapter_emergency | binary | neg_risk
    resolved_id      String,
    settled_price    Nullable(String),
    payouts          Nullable(String),          -- comma-separated payout numerators
    tx_hash          String,
    log_index        UInt64,
    block_number     UInt64,
    block_timestamp  UInt64
) ENGINE = ReplacingMergeTree
PARTITION BY toYYYYMM(toDateTime(block_timestamp))
ORDER BY (resolved_id, block_timestamp, id);
