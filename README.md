# Polymarket Substreams Packages

A collection of Substreams packages for extracting and processing events from Polymarket smart contracts on the Polygon network.

## Overview

This project provides Substreams packages for three core Polymarket contracts:

| Package | Contract | Description |
|---------|----------|-------------|
| [polymarket-ctf](./polymarket-ctf) | Conditional Tokens Framework | Manages conditional token creation and transfers |
| [polymarket-exchange](./polymarket-exchange) | CTF Exchange | Handles order matching and trading |
| [polymarket-neg-risk-ctf](./polymarket-neg-risk-ctf) | Negative Risk CTF | Specialized CTF for negative risk markets |

## Architecture

All packages follow a unified design pattern:

- **Single-pass event extraction** - Efficiently extracts all events in one block traversal
- **Optimized address comparison** - Uses `alloy_primitives::Address` for fast contract filtering
- **Consistent protobuf schema** - All events include `TransactionContext` metadata
- **Modular outputs** - Separate modules for different event categories

## Quick Start

### Prerequisites

- Rust with `wasm32-unknown-unknown` target
- Substreams CLI (`substreams` command)
- Make (optional, for convenience)
- Substreams API key — authenticate once with `substreams auth` ([get a key at thegraph.market](https://thegraph.market))

### Build All Packages

```bash
make build-all
```

### Package All SPKGs

```bash
make package-all
```

### Run a Package

```bash
# CTF package
make run-ctf

# Exchange package
make run-exchange

# Negative Risk CTF package
make run-neg-risk

# Run a specific module
make run-exchange MODULE=map_exchange_events
```

## Contract Addresses

| Contract | Address | Explorer |
|----------|---------|----------|
| CTF | `0x4D97DCd97eC945f40cF65F87097ACe5EA0476045` | [Polygonscan](https://polygonscan.com/address/0x4D97DCd97eC945f40cF65F87097ACe5EA0476045) |
| Exchange | `0x4bFb41d5B3570DeFd03C39a9A4D8dE6Bd8B8982E` | [Polygonscan](https://polygonscan.com/address/0x4bFb41d5B3570DeFd03C39a9A4D8dE6Bd8B8982E) |
| Neg Risk CTF | `0xc5d563a36ae78145c45a50134d48a1215220f80a` | [Polygonscan](https://polygonscan.com/address/0xc5d563a36ae78145c45a50134d48a1215220f80a) |

## Project Structure

```
substreams-polymarket/
├── Makefile                  # Unified build/run commands
├── polymarket-ctf/           # Conditional Tokens Framework
│   ├── src/
│   ├── proto/
│   ├── substreams.yaml
│   └── Cargo.toml
├── polymarket-exchange/      # CTF Exchange
│   ├── src/
│   ├── proto/
│   ├── substreams.yaml
│   └── Cargo.toml
└── polymarket-neg-risk-ctf/  # Negative Risk CTF
    ├── src/
    ├── proto/
    ├── substreams.yaml
    └── Cargo.toml
```

## Dependencies

All packages use consistent dependency versions:

- `substreams`: ^0.7
- `substreams-ethereum`: ^0.11
- `ethabi`: ^18

## Available Make Commands

```bash
# Build individual packages
make build-ctf
make build-exchange
make build-neg-risk

# Build all packages
make build-all

# Package individual SPKGs
make package-ctf
make package-exchange
make package-neg-risk

# Package all SPKGs
make package-all

# Run individual packages (last 1000 blocks, Polygon)
make run-ctf
make run-exchange
make run-neg-risk

# Run a specific module
make run-exchange MODULE=map_exchange_events

# Interactive GUI (substreams gui)
make gui-ctf
make gui-exchange
make gui-neg-risk

# Clean build artifacts
make clean
```

## Event Categories

### CTF Events
- `ConditionPreparation` / `ConditionResolution` — condition lifecycle
- `PositionSplit` / `PositionsMerge` — collateral and position management
- `PayoutRedemption` — winning position redemption
- ERC-1155: `TransferSingle`, `TransferBatch`, `ApprovalForAll`

### Exchange Events
- `OrderFilled`, `OrderCancelled`, `OrdersMatched` — order lifecycle
- `TokenRegistered` — token registry
- `FeeCharged` — protocol fees

### Negative Risk CTF Events
- Fee: `FeeCharged`
- Admin: `NewAdmin`, `NewOperator`, `RemovedAdmin`, `RemovedOperator`
- Trading: `OrderFilled`, `OrderCancelled`, `OrdersMatched`
- Registry: `TokenRegistered`
- Pause: `TradingPaused`, `TradingUnpaused`
- Config: `ProxyFactoryUpdated`, `SafeFactoryUpdated`

## Resources

- [Substreams Documentation](https://docs.streamingfast.io/substreams)
- [Polymarket](https://polymarket.com)
- [Polygon Network](https://polygon.technology)

## License

MIT
