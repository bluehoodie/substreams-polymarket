# Polymarket Wallet Factory Substreams Package

Substreams package for extracting events from the Polymarket DepositWalletFactory contract on Polygon.

## Contract Information

- **Address**: `0x00000000000Fb5C9ADea0298D729A0CB3823Cc07`
- **Network**: Polygon
- **Initial Block**: 84902000
- **Explorer**: [View on Polygonscan](https://polygonscan.com/address/0x00000000000Fb5C9ADea0298D729A0CB3823Cc07)

## Available Modules

| Module | Description | Output Type |
|--------|-------------|-------------|
| `map_factory_events` | Extracts wallet-factory events | `proto:polymarket.wallet_factory.v1.FactoryEvents` |

## Quick Start

### Build the WASM binary:

```bash
make build-wallet-factory
# or
cd polymarket-wallet-factory && substreams build
```

### Create the Substreams package:

```bash
cd polymarket-wallet-factory && substreams pack
```

### Run the Substreams:

```bash
substreams run substreams.yaml map_factory_events \
  --network polygon \
  --start-block 84902000 \
  --stop-block +10000
```

## Event Types

The package extracts the following events:

### Factory Events
- `WalletDeployed` - A deposit wallet was deployed (`wallet`, `owner`, `id`, `implementation`)
- `ImplementationAuthorized` - A wallet implementation was authorized

## Dependencies

- `substreams`: ^0.7
- `substreams-ethereum`: ^0.11
- `ethabi`: ^18

## Binary Output

Build artifacts: `target/wasm32-unknown-unknown/release/polymarket_wallet_factory.wasm`
