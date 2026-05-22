# Polymarket Substreams Packages

A collection of Substreams packages for extracting and processing events from Polymarket smart contracts on the Polygon network.

## Overview

| Package | Contract | V2 Address | Deploy Block |
|---------|----------|-----------|--------------|
| [polymarket-ctf](./polymarket-ctf) | Conditional Tokens Framework | `0x4D97DCd97eC945f40cF65F87097ACe5EA0476045` (unchanged) | — |
| [polymarket-exchange](./polymarket-exchange) | CTF Exchange V2 | `0xE111180000d2663C0091e4f400237545B87B996B` | 84902353 |
| [polymarket-neg-risk-ctf](./polymarket-neg-risk-ctf) | Neg Risk CTF Exchange V2 | `0xe2222d279d744050d28e00520010520000310F59` | 85058176 |
| [polymarket-neg-risk-adapter](./polymarket-neg-risk-adapter) | NegRiskAdapter | `0xd91E80cF2E7be2e162c6513ceD06f1dD0dA35296` | 50505403 |
| [polymarket-collateral](./polymarket-collateral) | pUSD + Collateral Adapters | Multiple (see below) | 84902320 |
| [polymarket-wallet-factory](./polymarket-wallet-factory) | DepositWalletFactory | `0x00000000000Fb5C9ADea0298D729A0CB3823Cc07` | 84902000 |

### Collateral Package Addresses

| Contract | Address |
|----------|---------|
| pUSD | `0xC011a7E12a19f7B1f670d46F03B03f3342E82DFB` |
| CollateralOnramp | `0x93070a847efEf7F70739046A929D47a521F5B8ee` |
| CtfCollateralAdapter | `0xAdA100Db00Ca00073811820692005400218FcE1f` |
| NegRiskCtfCollateralAdapter | `0xadA2005600Dec949baf300f4C6120000bDB6eAab` |

## Architecture

All packages follow a unified design pattern:

- **Single-pass event extraction** — all events extracted in one block traversal
- **Byte-level address comparison** — 20-byte array for fast contract filtering (~25x faster than string)
- **Consistent protobuf schema** — all events include `TransactionContext` metadata
- **Modular outputs** — separate modules for different event categories

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
