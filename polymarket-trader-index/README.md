# Polymarket Trader Index Substreams Package

Foundational **block index** of Polymarket user activity. Emits a `user:<address>`
key for every wallet that took *any* action in a block — trades, position
splits/merges, redemptions, conversions, pUSD transfers, ERC-1155 transfers, and
wallet deployments — across **all** Polymarket contracts. Downstream packages import
this and add a `blockFilter` querying `user:0x…` to skip every block a wallet never
touched.

## Contract Information

- **Network**: Polygon
- **Initial Block**: 4023686 (CTF genesis)
- **Contracts covered**: all 8 Polymarket contracts (exchange, neg-risk CTF exchange, CTF, neg-risk adapter, pUSD, both collateral adapters, wallet factory)

## Available Modules

| Module | Kind | Description | Output Type |
|--------|------|-------------|-------------|
| `map_user_keys` | map | Warming gate — decodes Polymarket logs and emits a `user:<address>` key per active wallet | `proto:sf.substreams.index.v1.Keys` |
| `index_users` | blockIndex | The block index consumers filter against (`user:0x…`) | `proto:sf.substreams.index.v1.Keys` |
| `map_user_activity` | map | Given a `user:0x…` query, emits the watched wallets active in each surviving block | `proto:sf.substreams.index.v1.Keys` |

## Key Namespace

| Key | Meaning |
|-----|---------|
| `user:<address>` | this wallet took **any** action on Polymarket in the block, across **all** contracts |

Keys are lowercase hex with a `0x` prefix. A consumer filters with e.g.
`query.string: "user:0xabc… || user:0xdef…"` to process only the blocks those
wallets were active in.

## Quick Start

### Build the WASM binary:

```bash
make build-trader-index
# or
cd polymarket-trader-index && substreams build
```

### Create the Substreams package:

```bash
cd polymarket-trader-index && substreams pack
```

### Run the Substreams:

`map_user_activity` takes a `user:0x…` param naming the wallet(s) to watch:

```bash
substreams run substreams.yaml map_user_activity \
  -p map_user_activity="user:0xefe62ed1df3f4afcc8a2afe2e4e8fcde965c9520" \
  --network polygon \
  --start-block 4023686 \
  --stop-block +1000000
```

Pass an `||` list to watch several wallets at once.

> **Warm the index first.** `index_users` is versioned, so the first full-range
> `--production-mode` pass *computes* the index (and returns nothing). Once warm,
> filtered runs skip straight to the wallet's active blocks. See the
> `warming-substreams-indexes` workflow.

## Usage in Downstream Packages

Import this package and gate a `map`/`store` on `index_users`; the engine skips
every block none of the watched wallets touched, while the per-contract packages
supply the decoded events:

```yaml
imports:
  pmusers: ../polymarket-trader-index/polymarket-trader-index-v0.12.0.spkg

modules:
  - name: map_wallet_activity
    kind: map
    blockFilter:
      module: pmusers:index_users
      query: { params: true }        # e.g. "user:0xA || user:0xB"
```

## Dependencies

- `substreams`: ^0.7
- `substreams-ethereum`: ^0.11

## Binary Output

Build artifacts: `target/wasm32-unknown-unknown/release/polymarket_trader_index.wasm`
