# Polymarket Negative Risk CTF Substreams Package

Substreams package for extracting events from the Polymarket Negative Risk Conditional Tokens Framework contract on Polygon.

## Contract Information

- **Address**: `0xe2222d279d744050d28e00520010520000310F59`
- **Network**: Polygon
- **Explorer**: [View on Polygonscan](https://polygonscan.com/address/0xe2222d279d744050d28e00520010520000310F59)

## Available Modules

| Module | Description | Output Type |
|--------|-------------|-------------|
| `map_trading_events` | Extracts trading events | `proto:polymarket.neg_risk_ctf.v1.TradingEvents` |
| `map_fee_events` | Extracts fee-related events | `proto:polymarket.neg_risk_ctf.v1.FeeEvents` |
| `map_admin_events` | Extracts admin role events | `proto:polymarket.neg_risk_ctf.v1.AdminEvents` |
| `map_pause_events` | Extracts user pause events | `proto:polymarket.neg_risk_ctf.v1.PauseEvents` |
| `map_approval_events` | Extracts order approval events | `proto:polymarket.neg_risk_ctf.v1.OrderApprovalEvents` |
| `map_all_events` | Extracts all negative risk CTF events | `proto:polymarket.neg_risk_ctf.v1.AllEvents` |

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

The package extracts the following V2 event categories:

### Trading Events
- `OrderFilled` - Order filled (with `side` uint8, `token_id`, `builder`, `metadata`)
- `OrdersMatched` - Orders matched (with `side` uint8, `token_id`)

### Fee Events
- `FeeCharged` - Protocol fee collected (`recipient`, `amount`)
- `FeeReceiverUpdated` - Fee receiver address changed
- `MaxFeeRateUpdated` - Maximum fee rate changed

### Admin Events
- `NewAdmin` - New admin added
- `NewOperator` - New operator added
- `RemovedAdmin` - Admin removed
- `RemovedOperator` - Operator removed

### Pause Events
- `UserPaused` - User paused from trading
- `UserUnpaused` - User unpaused
- `UserPauseBlockIntervalUpdated` - Pause interval changed

### Order Approval Events
- `OrderPreapproved` - Order preapproved
- `OrderPreapprovalInvalidated` - Order preapproval invalidated

## Dependencies

- `substreams`: ^0.7
- `substreams-ethereum`: ^0.11
- `ethabi`: ^18

## Binary Output

Build artifacts: `target/wasm32-unknown-unknown/release/polymarket_neg_risk_ctf.wasm`
