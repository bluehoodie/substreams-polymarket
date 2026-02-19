# Polymarket Exchange Substreams Package

Substreams package for extracting events from the Polymarket CTF Exchange contract on Polygon.

## Contract Information

- **Address**: `0x4bFb41d5B3570DeFd03C39a9A4D8dE6Bd8B8982E`
- **Network**: Polygon
- **Explorer**: [View on Polygonscan](https://polygonscan.com/address/0x4bFb41d5B3570DeFd03C39a9A4D8dE6Bd8B8982E)

## Available Modules

| Module | Description | Output Type |
|--------|-------------|-------------|
| `map_exchange_events` | Extracts exchange trading events | `proto:exchange.v1.ExchangeEvents` |
| `map_registry_events` | Extracts token registry events | `proto:exchange.v1.RegistryEvents` |
| `map_fee_events` | Extracts fee-related events | `proto:exchange.v1.FeeEvents` |
| `map_all_events` | Extracts all exchange events | `proto:exchange.v1.AllEvents` |

## Quick Start

### Build the WASM binary:

```bash
make build-exchange
# or
cd polymarket-exchange && substreams build
```

### Create the Substreams package:

```bash
make package-exchange
# or
cd polymarket-exchange && substreams pack
```

### Run the Substreams:

```bash
make run-exchange
# or
substreams run substreams.yaml map_all_events \
  --network polygon \
  --start-block -1000
```

## Event Types

The package extracts the following event categories:

### Exchange Events
- `OrderFilled` - Order executed
- `OrderCancelled` - Order cancelled
- `OrdersMatched` - Orders matched

### Registry Events
- `TokenRegistered` - New token pair registered

### Fee Events
- `FeeCharged` - Trading fee charged

## Dependencies

- `substreams`: ^0.7
- `substreams-ethereum`: ^0.11
- `ethabi`: ^18

## Binary Output

Build artifacts: `target/wasm32-unknown-unknown/release/polymarket_exchange.wasm`
