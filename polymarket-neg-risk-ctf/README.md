# Polymarket Negative Risk CTF Substreams Package

Substreams package for extracting events from the Polymarket Negative Risk Conditional Tokens Framework contract on Polygon.

## Contract Information

- **Address**: `0xe2222d279d744050d28e00520010520000310F59`
- **Network**: Polygon
- **Explorer**: [View on Polygonscan](https://polygonscan.com/address/0xe2222d279d744050d28e00520010520000310F59)

## Available Modules

| Module | Description | Output Type |
|--------|-------------|-------------|
| `map_all_events` | Extracts all negative risk CTF events | `proto:negriskctf.v1.AllEvents` |

## Quick Start

### Build the WASM binary:

```bash
make build-neg-risk
# or
cd polymarket-neg-risk-ctf && substreams build
```

### Create the Substreams package:

```bash
make package-neg-risk
# or
cd polymarket-neg-risk-ctf && substreams pack
```

### Run the Substreams:

```bash
make run-neg-risk
# or
substreams run substreams.yaml map_all_events \
  --network polygon \
  --start-block -1000
```

## Event Types

The package extracts the following event categories:

### Fee Events
- `FeeCharged` - Protocol fee collected

### Admin Events
- `NewAdmin` - New admin added
- `NewOperator` - New operator added
- `RemovedAdmin` - Admin removed
- `RemovedOperator` - Operator removed

### Trading Events
- `OrderCancelled` - Order cancelled
- `OrderFilled` - Order filled (main trading event)
- `OrdersMatched` - Taker order matched against maker orders

### Registry Events
- `TokenRegistered` - Token pair registered for trading

### Pause Events
- `TradingPaused` - Trading paused
- `TradingUnpaused` - Trading unpaused

### Config Events
- `ProxyFactoryUpdated` - Proxy factory updated
- `SafeFactoryUpdated` - Safe factory updated

## Dependencies

- `substreams`: ^0.7
- `substreams-ethereum`: ^0.11
- `ethabi`: ^18

## Binary Output

Build artifacts: `target/wasm32-unknown-unknown/release/polymarket_neg_risk_ctf.wasm`
