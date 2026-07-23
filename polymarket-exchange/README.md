# Polymarket Exchange Substreams Package

Substreams package for extracting events from the Polymarket CTF Exchange contract on Polygon.

## Contract Information

- **Address**: `0xE111180000d2663C0091e4f400237545B87B996B`
- **Network**: Polygon
- **Explorer**: [View on Polygonscan](https://polygonscan.com/address/0xE111180000d2663C0091e4f400237545B87B996B)

## Available Modules

| Module | Description | Output Type |
|--------|-------------|-------------|
| `map_exchange_events` | Extracts exchange trading events | `proto:polymarket.exchange.v1.ExchangeEvents` |
| `map_fee_events` | Extracts fee-related events | `proto:polymarket.exchange.v1.FeeEvents` |
| `map_admin_events` | Extracts admin role events | `proto:polymarket.exchange.v1.AdminEvents` |
| `map_pause_events` | Extracts user pause events | `proto:polymarket.exchange.v1.PauseEvents` |
| `map_approval_events` | Extracts order approval events | `proto:polymarket.exchange.v1.OrderApprovalEvents` |
| `map_all_events` | Extracts all exchange events | `proto:polymarket.exchange.v1.AllEvents` |

## Quick Start

### Build the WASM binary:

```bash
make build-exchange
# or
cd polymarket-exchange && substreams build
```

### Create the Substreams package:

```bash
cd polymarket-exchange && substreams pack
```

### Run the Substreams:

```bash
substreams run substreams.yaml map_all_events \
  --network polygon \
  --start-block -1000
```

## Event Types

The package extracts the following V2 event categories:

### Exchange Events
- `OrderFilled` - Order filled (with `side` uint8, `token_id`, `builder`, `metadata`)
- `OrdersMatched` - Orders matched (with `side` uint8, `token_id`)

### Fee Events
- `FeeCharged` - Trading fee charged (`recipient`, `amount`)
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

Build artifacts: `target/wasm32-unknown-unknown/release/polymarket_exchange.wasm`
