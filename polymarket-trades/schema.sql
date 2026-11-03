-- PostgreSQL schema for `db_out` (substreams sink postgres). Amounts are raw
-- 6-decimal integers (collateral and shares), prices are collateral per share with
-- 6 decimals. Block timestamps are unix seconds.

-- One row per order fill (OrderFilled log).
CREATE TABLE IF NOT EXISTS trades (
    id               TEXT PRIMARY KEY,          -- <tx_hash>-<log_index>
    token_id         TEXT        NOT NULL,
    side             SMALLINT    NOT NULL,      -- 0 = BUY, 1 = SELL
    shares           NUMERIC     NOT NULL,
    collateral       NUMERIC     NOT NULL,      -- fee included
    price            NUMERIC(20, 6) NOT NULL,
    fee              NUMERIC     NOT NULL,
    maker            TEXT        NOT NULL,
    taker            TEXT        NOT NULL,
    order_hash       TEXT        NOT NULL,
    exchange         TEXT        NOT NULL,      -- ctf_v2 | neg_risk_v2 | exchange_v3
    is_match         BOOLEAN     NOT NULL,
    is_volume        BOOLEAN     NOT NULL,      -- counts toward volume (each match once per token)
    tx_hash          TEXT        NOT NULL,
    log_index        BIGINT      NOT NULL,
    block_number     BIGINT      NOT NULL,
    block_timestamp  BIGINT      NOT NULL
);
CREATE INDEX IF NOT EXISTS trades_token_time ON trades (token_id, block_timestamp);
CREATE INDEX IF NOT EXISTS trades_order_hash ON trades (order_hash);
CREATE INDEX IF NOT EXISTS trades_maker ON trades (maker);

-- Running summary per outcome token, each block that trades the token replaces
-- its row with the cumulative values from map_market_stats.
CREATE TABLE IF NOT EXISTS market_stats (
    token_id                 TEXT PRIMARY KEY,
    trades                   BIGINT  NOT NULL,
    buys                     BIGINT  NOT NULL,
    sells                    BIGINT  NOT NULL,
    volume_collateral        NUMERIC NOT NULL,
    volume_shares            NUMERIC NOT NULL,
    fees                     NUMERIC NOT NULL,
    last_price               NUMERIC(20, 6),
    block_vwap               NUMERIC(20, 6),
    block_volume_collateral  NUMERIC NOT NULL,
    block_volume_shares      NUMERIC NOT NULL,
    last_trade_block         BIGINT  NOT NULL,
    last_trade_timestamp     BIGINT  NOT NULL
);

-- 1-minute OHLCV per token. Each block's fragment is merged in place: open is set
-- once (first block), close by the latest block, high = GREATEST, low = LEAST,
-- volumes and trades added.
CREATE TABLE IF NOT EXISTS candles_1m (
    token_id           TEXT    NOT NULL,
    bucket_start       BIGINT  NOT NULL,        -- unix seconds, multiple of 60
    open               NUMERIC(20, 6),
    high               NUMERIC(20, 6),
    low                NUMERIC(20, 6),
    close              NUMERIC(20, 6),
    volume_shares      NUMERIC NOT NULL DEFAULT 0,
    volume_collateral  NUMERIC NOT NULL DEFAULT 0,
    trades             BIGINT  NOT NULL DEFAULT 0,
    first_block        BIGINT,
    last_block         BIGINT,
    PRIMARY KEY (token_id, bucket_start)
);

-- Fill progress per signed order, summed across blocks.
CREATE TABLE IF NOT EXISTS order_fills (
    order_hash           TEXT PRIMARY KEY,
    owner                TEXT,
    token_id             TEXT,
    side                 SMALLINT,
    exchange             TEXT,
    filled_shares        NUMERIC NOT NULL DEFAULT 0,
    filled_collateral    NUMERIC NOT NULL DEFAULT 0,
    fees                 NUMERIC NOT NULL DEFAULT 0,
    fills                BIGINT  NOT NULL DEFAULT 0,
    first_fill_block     BIGINT,
    last_fill_block      BIGINT,
    last_fill_timestamp  BIGINT
);
CREATE INDEX IF NOT EXISTS order_fills_owner ON order_fills (owner);
CREATE INDEX IF NOT EXISTS order_fills_token ON order_fills (token_id);

-- The full signed orders behind the fills (map_orders, from matchOrders calldata),
-- one row per order. filled sums the maker-asset amount of every settlement
-- (collateral for a BUY, shares for a SELL), the same unit as maker_amount.
CREATE TABLE IF NOT EXISTS orders (
    order_hash      TEXT PRIMARY KEY,
    exchange        TEXT,
    maker           TEXT,
    signer          TEXT,
    token_id        TEXT,
    side            SMALLINT,
    maker_amount    NUMERIC,
    taker_amount    NUMERIC,
    limit_price     TEXT,
    size_shares     NUMERIC,
    signature_type  SMALLINT,
    created_at_ms   BIGINT,
    metadata        TEXT,
    builder         TEXT,
    filled          NUMERIC NOT NULL DEFAULT 0,
    fees            NUMERIC NOT NULL DEFAULT 0,
    settlements     BIGINT  NOT NULL DEFAULT 0,
    first_block     BIGINT,
    last_block      BIGINT,
    last_timestamp  BIGINT
);
CREATE INDEX IF NOT EXISTS orders_token ON orders (token_id);
CREATE INDEX IF NOT EXISTS orders_maker ON orders (maker);

-- What each observed order has left, as the exchange tracks it (maker_amount
-- minus filled), and the same in shares. An upper bound on the book: orders
-- cancelled off-chain or never filled are not here.
CREATE OR REPLACE VIEW order_book AS
SELECT o.*,
       GREATEST(o.maker_amount - o.filled, 0) AS remaining,
       CASE WHEN o.side = 0
            THEN TRUNC(GREATEST(o.maker_amount - o.filled, 0) * o.taker_amount / NULLIF(o.maker_amount, 0))
            ELSE GREATEST(o.maker_amount - o.filled, 0)
       END AS remaining_shares,
       o.filled >= o.maker_amount AS fully_filled
FROM orders o;

-- Settled outcomes: UMA CTF adapter QuestionResolved / QuestionEmergencyResolved
-- (resolved_id = question id) and Protocol V2 module ConditionResolved
-- (resolved_id = condition id).
CREATE TABLE IF NOT EXISTS resolutions (
    id               TEXT PRIMARY KEY,          -- <tx_hash>-<log_index>
    source           TEXT   NOT NULL,           -- uma_adapter | uma_adapter_emergency | binary | neg_risk
    resolved_id      TEXT   NOT NULL,
    settled_price    TEXT,                      -- UMA settled price, NULL for module and emergency resolutions
    payouts          TEXT,                      -- comma-separated payout numerators
    tx_hash          TEXT   NOT NULL,
    log_index        BIGINT NOT NULL,
    block_number     BIGINT NOT NULL,
    block_timestamp  BIGINT NOT NULL
);
CREATE INDEX IF NOT EXISTS resolutions_resolved_id ON resolutions (resolved_id);

-- Larger candle intervals, rolled up from candles_1m. open/close come from the
-- first/last minute in the interval.
CREATE OR REPLACE VIEW candles_5m AS
SELECT token_id,
       bucket_start - bucket_start % 300 AS bucket_start,
       (ARRAY_AGG(open ORDER BY bucket_start))[1] AS open,
       MAX(high) AS high,
       MIN(low) AS low,
       (ARRAY_AGG(close ORDER BY bucket_start DESC))[1] AS close,
       SUM(volume_shares) AS volume_shares,
       SUM(volume_collateral) AS volume_collateral,
       SUM(trades) AS trades
FROM candles_1m
GROUP BY token_id, bucket_start - bucket_start % 300;

CREATE OR REPLACE VIEW candles_1h AS
SELECT token_id,
       bucket_start - bucket_start % 3600 AS bucket_start,
       (ARRAY_AGG(open ORDER BY bucket_start))[1] AS open,
       MAX(high) AS high,
       MIN(low) AS low,
       (ARRAY_AGG(close ORDER BY bucket_start DESC))[1] AS close,
       SUM(volume_shares) AS volume_shares,
       SUM(volume_collateral) AS volume_collateral,
       SUM(trades) AS trades
FROM candles_1m
GROUP BY token_id, bucket_start - bucket_start % 3600;

CREATE OR REPLACE VIEW candles_1d AS
SELECT token_id,
       bucket_start - bucket_start % 86400 AS bucket_start,
       (ARRAY_AGG(open ORDER BY bucket_start))[1] AS open,
       MAX(high) AS high,
       MIN(low) AS low,
       (ARRAY_AGG(close ORDER BY bucket_start DESC))[1] AS close,
       SUM(volume_shares) AS volume_shares,
       SUM(volume_collateral) AS volume_collateral,
       SUM(trades) AS trades
FROM candles_1m
GROUP BY token_id, bucket_start - bucket_start % 86400;
