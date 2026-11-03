# Polymarket Trades Substreams Package

A trade tape for Polymarket on Polygon: one `Trade` per order fill across all three
exchanges, with shares, fee-aware collateral and price. It is the base of the
order-book-style views (market stats, candles, per-order fills) built on top of it.

Chain data only carries **matched** orders. Resting limit orders, cancellations and
book depth live in Polymarket's off-chain CLOB, so this is a tape of what traded, not a
live book.

## Sources

| Exchange | Address | `exchange` |
|----------|---------|------------|
| CTF Exchange V2 | `0xE111180000d2663C0091e4f400237545B87B996B` | `ctf_v2` |
| Neg Risk CTF Exchange V2 | `0xe2222d279d744050d28e00520010520000310F59` | `neg_risk_v2` |
| Protocol V2 Exchange (ExchangeV3) | `0xe3333700cA9d93003F00f0F71f8515005F6c00Aa` | `exchange_v3` |

Apart from `map_orders`, which reads settlement calldata, the package decodes no
blocks. Its manifest imports the sibling
`../polymarket-exchange` and `../polymarket-neg-risk-ctf` manifests and reads
`exchange:map_exchange_events` and `neg_risk_ctf:map_trading_events`. Built from the
same commit and toolchain as the published packages of the same version, those modules
hash the same, so their cached outputs are reused.

## Modules

| Module | Description | Output Type |
|--------|-------------|-------------|
| `map_trades` | One trade per `OrderFilled`, in block order | `proto:polymarket.trades.v1.Trades` |
| `store_token_totals` | Running per-token totals (`<token_id>:<field>`): trades, buys, sells, volume_collateral, volume_shares, fees | store (add, bigint) |
| `store_last_price` | Last traded price per token | store (set, string) |
| `map_market_stats` | One `TokenStats` per token traded in the block: running totals, last price, block VWAP and volume | `proto:polymarket.trades.v1.MarketStats` |
| `map_candles` | 1-minute OHLCV fragment per token from the block's `is_volume` trades | `proto:polymarket.trades.v1.Candles` |
| `db_out` | PostgreSQL rows for `substreams sink postgres` (`schema.sql`) | `proto:sf.substreams.sink.database.v1.DatabaseChanges` |
| `db_out_clickhouse` | ClickHouse rows for `substreams sink clickhouse` (`clickhouse-schema.sql`) | `proto:sf.substreams.sink.database.v1.DatabaseChanges` |
| `map_order_fills` | One `OrderFill` per order filled in the block: owner, token, side, and that block's filled shares, collateral, fees and fill count | `proto:polymarket.trades.v1.OrderFills` |
| `map_orders` | The full signed order behind each fill, decoded from `matchOrders` calldata: full size, limit price, signer, creation time, and this settlement's fill | `proto:polymarket.trades.v1.ObservedOrders` |

The stores start at block 84902353, so `map_market_stats` needs that backfill before
its totals are complete.

## Candles

`map_candles` keeps no store, since one key per candle would grow without bound.
Each block emits one `Candle` per token with `is_volume` trades. It covers the
1-minute bucket that holds the block timestamp (`bucket_start`, unix seconds), and
its prices are fee-aware trade prices. A bucket spans many blocks, so the sink merges
the fragments of the same `(token_id, bucket_start)` like this:

| Field | Merge |
|-------|-------|
| `open` | from the fragment with the lowest `(block_number, open_log_index)` (ClickHouse `argMin`) |
| `close` | from the fragment with the highest `(block_number, close_log_index)` (`argMax`) |
| `high` / `low` | max / min (Postgres upsert with `GREATEST` / `LEAST`) |
| `volume_shares`, `volume_collateral`, `trades` | sum |

Larger intervals (5m, 1h, 1d) are rolled up from the 1-minute candles in the sink,
with the same rule.

## Order fills

`map_order_fills` keeps no store either. A store per order would hold one key set
for every order ever filled: over blocks 96,590,000–96,600,000 that was 395,523
orders and about 1.77M keys, while 94.5% of orders finished within one block. Each
block emits one `OrderFill` per order filled in it, and the sink keeps the running
total per `order_hash`: `shares`, `collateral`, `fees` and `fills` summed, first fill
block = min `block_number`, last = max. Chain data has no full order size, so this is
what was filled, not what remains.

## Observed orders

`OrderFilled` carries only what was filled. The full signed order is in the input of
the operator's settlement call, so `map_orders` decodes it from the block's calls
(successful, non-reverted calls to the three exchanges). Every settlement seen on
blocks 96600000 and 96680009 was a top-level call from an operator EOA:

| Exchange | Function | Selector | EIP-712 domain |
|----------|----------|----------|----------------|
| CTF Exchange V2, Neg Risk CTF Exchange V2 | `matchOrders(bytes32,Order,Order[],uint256,uint256[],uint256,uint256[])` | `0x3c2b4399` | "Polymarket CTF Exchange", "2" |
| ExchangeV3 | `matchOrders(Order,Order[],uint256[],uint256[],TakerAmounts)` | `0x0fb25ba4` | "Polymarket CTF Exchange", "3" |
| ExchangeV3 | `matchOrdersAndPrepareCombinatorial(…, uint256[])` | `0xe75cf6e5` | same |

`order_hash` is the EIP-712 hash of the order (chain id 137, the exchange's own
address), the same `orderHash` as in `OrderFilled`. `fill_amount` equals that
OrderFilled's `makerAmountFilled`. V2 orders have no expiration or fee rate: the
operator sets each fill's fee.

The book view is a sink join, not a store (same reason as order fills): sum
`fill_amount` per `order_hash`, and `maker_amount` minus that sum is what the order
has left, in maker-asset units (collateral for a BUY, shares for a SELL), the way the
exchange tracks it. It is an upper bound: orders cancelled off-chain or never filled
are not on chain.

## Trade fields

- `side`: side of the filled order, `0` = BUY (pays collateral for shares), `1` = SELL.
- `shares`, `collateral`, `fee`: raw 6-decimal integers. The fee is in collateral: a
  BUY pays it on top (`collateral = maker_amount_filled + fee`), a SELL has it taken
  from the proceeds (`collateral = taker_amount_filled − fee`).
- `price`: `collateral / shares` rounded down to 6 decimals, so it is the price this
  order's owner actually paid or got, fee included.
- `is_match`: true for the taker order's own fill (its `taker` is the exchange itself).
  Every match has exactly one, so summing `is_match` trades counts each match once.
  Maker fills of the same match carry the complementary view.
- `is_volume`: true for the fills that count toward per-token volume, so each match
  is counted once per token. That is the taker fill, plus any maker fill on a
  different token than the taker's (a complementary match that mints or merges, so
  the taker fill doesn't cover that token). Maker fills on the taker's own token are
  the other side of the taker fill and are not volume. Market stats count trades and
  volume from `is_volume` fills only, and fees from every fill.
- `token_id`: for `exchange_v3`, a Protocol V2 PositionManager position id, not a
  Conditional Tokens ERC1155 id.

## SQL sink

`db_out` writes six tables to PostgreSQL:

| Table | How rows are written |
|-------|----------------------|
| `trades` | inserted, one per fill, id `<tx_hash>-<log_index>` |
| `market_stats` | upserted per token with the absolute values from `map_market_stats` |
| `candles_1m` | merged per `(token_id, bucket_start)`: `open` set once, `close` set by each block, `high` `GREATEST`, `low` `LEAST`, volumes and `trades` added |
| `order_fills` | merged per `order_hash`: amounts and `fills` added, first fill block set once, last fill block set by each block |
| `orders` | merged per `order_hash` from `map_orders`: order fields set once, `filled`, `fees` and `settlements` added. The `order_book` view adds `remaining`, `remaining_shares` and `fully_filled` |
| `resolutions` | inserted: UMA adapter `QuestionResolved` / `QuestionEmergencyResolved` and Protocol V2 module `ConditionResolved`, from `polymarket-resolution` |

`schema.sql` also has `candles_5m`, `candles_1h` and `candles_1d` views rolled up
from `candles_1m`.

```bash
make build-trades
export SUBSTREAMS_SINK_DSN="postgres://user:pass@localhost:5432/polymarket?sslmode=disable"
substreams sink postgres setup polymarket-trades/substreams.yaml
substreams sink postgres polymarket-trades/substreams.yaml -e polygon.streamingfast.io:443
```

The `add`/`max`/`min`/`set_if_null` upserts are PostgreSQL-only, so ClickHouse has its
own module, `db_out_clickhouse`, which only inserts. Candle and order fill fragments
land as rows, one per token or order per block. `candles_1m_mv` merges the candles
into the `candles_1m_agg` AggregatingMergeTree, and you read them from
`candles_1m_merged` and the `candles_5m` / `candles_1h` / `candles_1d` views. Order
totals are in the `order_fills_totals` view, and observed orders with what they
have left in the `order_book` view. Run the ClickHouse sink on final blocks,
because the materialized view can't take back rows from a reorg.

`substreams.clickhouse.yaml` imports `substreams.yaml`, so module hashes and caches
are shared, and points the sink at `db_out_clickhouse` and `clickhouse-schema.sql`:

```bash
export SUBSTREAMS_SINK_DSN="clickhouse://default:@localhost:9000/default"
substreams sink clickhouse setup polymarket-trades/substreams.clickhouse.yaml
substreams sink clickhouse polymarket-trades/substreams.clickhouse.yaml -e polygon.streamingfast.io:443 --final-blocks-only
```

`market_stats` comes from stores that start at block 84902353, so a sink started
later than that still backfills those stores from 84902353 first.

## Build and run

```bash
make build-trades          # builds polymarket-exchange and polymarket-neg-risk-ctf first
substreams run polymarket-trades/substreams.yaml map_trades -s 96600000 -t +100
```
