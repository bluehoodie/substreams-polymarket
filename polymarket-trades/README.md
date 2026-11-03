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

The package decodes no blocks. Its manifest imports the sibling
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

The stores start at block 84902353, so `map_market_stats` needs that backfill before
its totals are complete.

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

## Build and run

```bash
make build-trades          # builds polymarket-exchange and polymarket-neg-risk-ctf first
substreams run polymarket-trades/substreams.yaml map_trades -s 96600000 -t +100
```
