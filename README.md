# Polymarket Substreams Packages

A collection of Substreams packages for extracting and processing events from Polymarket smart contracts on the Polygon network.

## Overview

| Package | Contract | Address | Deploy Block |
|---------|----------|---------|--------------|
| [polymarket-ctf](./polymarket-ctf) | Conditional Tokens Framework | `0x4D97DCd97eC945f40cF65F87097ACe5EA0476045` | — |
| [polymarket-exchange](./polymarket-exchange) | CTF Exchange V2 | `0xE111180000d2663C0091e4f400237545B87B996B` | 84902353 |
| [polymarket-neg-risk-ctf](./polymarket-neg-risk-ctf) | Neg Risk CTF Exchange V2 | `0xe2222d279d744050d28e00520010520000310F59` | 85058176 |
| [polymarket-neg-risk-adapter](./polymarket-neg-risk-adapter) | NegRiskAdapter | `0xd91E80cF2E7be2e162c6513ceD06f1dD0dA35296` | 50505403 |
| [polymarket-collateral](./polymarket-collateral) | pUSD + Collateral Adapters | Multiple (see below) | 84902320 |
| [polymarket-resolution](./polymarket-resolution) | UMA Oracle + CTF Adapter | Multiple (see below) | 29786052 |
| [polymarket-wallet-factory](./polymarket-wallet-factory) | DepositWalletFactory | `0x00000000000Fb5C9ADea0298D729A0CB3823Cc07` | 84902000 |
| [polymarket-trader-index](./polymarket-trader-index) | All contracts (foundational user-activity index) | — | 4023686 |

### Collateral Package Addresses

| Contract | Address |
|----------|---------|
| pUSD | `0xC011a7E12a19f7B1f670d46F03B03f3342E82DFB` |
| CollateralOnramp | `0x93070a847efEf7F70739046A929D47a521F5B8ee` |
| CtfCollateralAdapter | `0xAdA100Db00Ca00073811820692005400218FcE1f` |
| NegRiskCtfCollateralAdapter | `0xadA2005600Dec949baf300f4C6120000bDB6eAab` |

### Resolution Package Addresses

| Contract | Address |
|----------|---------|
| UMA Oracle V2 | `0xee3afe347D5C74317041E2618C49534dAf887c24` |
| UMA Oracle V3 | `0x5953f2538F613E05bAED8A5AeFa8e6622467AD3D` |
| CTF Adapter V2 | `0x6A9D222616C90FcA5754cd1333cFD9b7fb6a4F74` |
| CTF Adapter V3 | `0x2f5e3684cb1F318eC51b00eDba38D79ac2c0aA9D` |

## Architecture

All packages follow a unified design pattern:

- **Single-pass event extraction** — all events extracted in one block traversal
- **Byte-level address comparison** — 20-byte array for fast contract filtering (~25x faster than string)
- **Consistent protobuf schema** — all events include `TransactionContext` metadata
- **Modular outputs** — separate modules for different event categories

## Block Indexes & Filtering

Every package ships a **block index** so the Substreams engine can *skip blocks it never needs to read*. Polymarket activity is sparse relative to all of Polygon, so this is the biggest lever on backfill speed and cost: a contract active in 0.5% of blocks costs roughly 0.5% as much to stream, and filtering by a single wallet is far more selective still.

A block index emits a set of string **keys** per block (`sf.substreams.index.v1.Keys`). A downstream `map`/`store` module declares a `blockFilter` with a query over those keys; **blocks whose keys don't match are never read, decoded, or processed**.

### Key namespaces

| Key | Emitted by | Meaning |
|-----|-----------|---------|
| `evt_addr:<address>` | every package — `index_events` | the block contains a log from this targeted contract |
| `trader:<address>` | `polymarket-exchange`, `polymarket-neg-risk-ctf` — `index_events` | this wallet was a maker/taker in an `OrderFilled`/`OrdersMatched` in the block |
| `user:<address>` | `polymarket-trader-index` — `index_users` | this wallet took **any** action on Polymarket in the block (trades, splits/merges, redemptions, conversions, pUSD transfers, ERC-1155 transfers, wallet deploys), across **all** contracts |

All keys are lowercase hex with a `0x` prefix.

### Query syntax (SQE)

A `blockFilter` query is a boolean expression over keys — `&&` (and), `||` (or), `-` (not), `( )` grouping. It can be hard-coded in the manifest (`query.string:`) or supplied at run time (`query.params: true`, read from the module's `params` input).

> The index alone changes nothing — a module skips blocks **only** when it declares a `blockFilter`. The query namespace must match the emitted keys exactly (`evt_addr:` vs `trader:` vs `user:`); a mismatch silently matches no blocks.

### Example use-cases

**1. Stream a single contract's events.** Every `map_*` module is already wired to its package's `index_events` (`evt_addr:`), so backfills skip irrelevant blocks automatically:

```bash
substreams run polymarket-collateral/substreams.yaml map_pusd_events -s 85049190 -t +500000
# only blocks containing a pUSD log are processed
```

**2. Track one trader's fills.** The exchange packages expose a params-driven `map_user_trades` backed by the `trader:` index:

```bash
substreams run polymarket-exchange/substreams.yaml map_user_trades \
  -p map_user_trades="trader:0xabc…" -s 84934480 -t +1000000
```

**3. Track an array of wallets across an exchange.** Pass an `||` list — the engine skips every block none of them traded in:

```bash
substreams run polymarket-neg-risk-ctf/substreams.yaml map_user_trades \
  -p map_user_trades="trader:0xabc… || trader:0xdef… || trader:0x123…"
```

**4. Track a wallet's entire Polymarket footprint.** `polymarket-trader-index` indexes user activity across *all* contracts from CTF genesis. A downstream package imports it and filters on `user:`, so a wallet that appears in only a few hundred of Polygon's ~88M blocks is streamed by reading only those blocks:

```yaml
imports:
  pmusers: ./polymarket-trader-index/polymarket-trader-index-v0.1.0.spkg
modules:
  - name: my_wallet_activity
    kind: map
    blockFilter:
      module: pmusers:index_users
      query: { params: true }          # "user:0xA || user:0xB || user:0xC"
    inputs:
      - params: string
      - source: sf.ethereum.type.v2.Block
    output:
      type: proto:my.types.WalletActivity
```

## Quick Start

### Prerequisites

- Rust with `wasm32-unknown-unknown` target
- Substreams CLI (`substreams` command)
- `buf` CLI (required for protobuf generation)
- Substreams API key — authenticate with `substreams auth` ([get a key at thegraph.market](https://thegraph.market))

### Build

```bash
# Build all packages
make build-all

# Build individual packages
make build-exchange
make build-ctf
make build-neg-risk-ctf
make build-neg-risk-adapter
make build-collateral
make build-resolution
make build-wallet-factory
```

### Run

```bash
# V2 exchange events (from V2 deploy block)
substreams run polymarket-exchange/substreams.yaml map_all_events \
  -s 84902353 -t +1000

# NegRisk adapter market events
substreams run polymarket-neg-risk-adapter/substreams.yaml map_market_events \
  -s 50505403 -t +10000

# pUSD wrap/unwrap events
substreams run polymarket-collateral/substreams.yaml map_pusd_events \
  -s 84902320 -t +10000

# Resolution pipeline (oracle + adapter events)
substreams run polymarket-resolution/substreams.yaml map_resolution_events \
  -s 29786052 -t +10000

# Wallet deployments
substreams run polymarket-wallet-factory/substreams.yaml map_factory_events \
  -s 84902000 -t +10000
```

## V2 Event Changes (exchange packages)

Polymarket launched V2 contracts in April 2026 with a redesigned order struct:

**`OrderFilled`**: `side` (uint8) + `token_id` replace the old `maker_asset_id`/`taker_asset_id`. New fields: `builder`, `metadata`.

**`OrdersMatched`**: Same restructuring — `side` (uint8) + `token_id` added, dual asset IDs removed.

**`FeeCharged`**: Removed `tokenId` parameter. V2 signature is `FeeCharged(address indexed recipient, uint256 amount)`.

**Removed**: `OrderCancelled`, `TokenRegistered` (V2 no longer emits these).

**New events**: `UserPaused`/`UserUnpaused`, `OrderPreapproved`/`OrderPreapprovalInvalidated`, `FeeReceiverUpdated`, `MaxFeeRateUpdated`, admin events.

## Event Categories

### Exchange / Neg Risk CTF (V2)
- Trading: `OrderFilled` (with `side`, `token_id`), `OrdersMatched`
- Fee: `FeeCharged`, `FeeReceiverUpdated`, `MaxFeeRateUpdated`
- Admin: `NewAdmin`, `NewOperator`, `RemovedAdmin`, `RemovedOperator`
- Pause: `UserPaused`, `UserUnpaused`, `UserPauseBlockIntervalUpdated`
- Approval: `OrderPreapproved`, `OrderPreapprovalInvalidated`

### CTF (unchanged)
- `ConditionPreparation`, `ConditionResolution`
- `PositionSplit`, `PositionsMerge`, `PayoutRedemption`
- ERC-1155: `TransferSingle`, `TransferBatch`, `ApprovalForAll`

### NegRiskAdapter
- Market: `MarketPrepared`, `QuestionPrepared`, `OutcomeReported`
- Trading: `PositionSplit`, `PositionsMerge`, `PositionsConverted`, `PayoutRedemption`
- Admin: `NewAdmin`, `RemovedAdmin`

### Resolution (UMA Oracle + CTF Adapter)
- Oracle: `ResolutionProposal`, `ResolutionDispute`, `ResolutionSettlement` (unified across V2/V3)
- Adapter: `QuestionInitialized`, `QuestionResolved`, `QuestionEmergencyResolved`, `QuestionFlagged`/`QuestionUnflagged`, `QuestionPaused`/`QuestionUnpaused`, `QuestionReset`, `AncillaryDataUpdated`
- Admin: `NewAdmin`, `RemovedAdmin`
- Alerts: `DisputeAlert` (disputes, emergency resolutions, flags, pauses, resets)

### Collateral
- pUSD: `Transfer`, `Wrapped`, `Unwrapped`
- Adapters: `PositionSplit`, `PositionsMerged`, `PositionsRedeemed`
- NegRisk adapter only: `PositionsConverted`

### Wallet Factory
- `WalletDeployed`, `ImplementationAuthorized`

## Dependencies

- `substreams`: ^0.7
- `substreams-ethereum`: ^0.11
- `ethabi`: ^18

## Resources

- [Substreams Documentation](https://docs.streamingfast.io/substreams)
- [Polymarket](https://polymarket.com)
- [Polygon Network](https://polygon.technology)

## License

MIT
