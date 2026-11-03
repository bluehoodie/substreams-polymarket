# Polymarket Exchange Substreams Package

Substreams package for extracting events from the Polymarket exchange contracts on Polygon:
CTF Exchange V2 and the Protocol V2 Exchange ("ExchangeV3").

## Contract Information

| Contract | Address | `exchange` field |
|----------|---------|------------------|
| CTF Exchange V2 | [`0xE111180000d2663C0091e4f400237545B87B996B`](https://polygonscan.com/address/0xE111180000d2663C0091e4f400237545B87B996B) | `ctf_v2` |
| Protocol V2 Exchange (ExchangeV3, proxy) | [`0xe3333700cA9d93003F00f0F71f8515005F6c00Aa`](https://polygonscan.com/address/0xe3333700cA9d93003F00f0F71f8515005F6c00Aa) | `exchange_v3` |

- **Network**: Polygon
- **Initial block**: 84902353 (CTF Exchange V2). ExchangeV3 was deployed after it (its address is listed in Polymarket's `polymarket-v2-external` README as of 2026-09-24; the exact deployment block isn't pinned here), so the initial block covers both.

Both contracts emit the same event signatures, so every module reads both and every
event carries an `exchange` field saying which one emitted it. ExchangeV3 `token_id`s are
Protocol V2 PositionManager position ids, not Conditional Tokens ERC1155 ids.

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

The package extracts the following event categories. ExchangeV3 emits the exchange, fee,
pause and order approval events; the admin events (and `FeeReceiverUpdated`,
`MaxFeeRateUpdated`) only come from CTF Exchange V2.

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

- `substreams`: ^0.8
- `substreams-ethereum`: ^0.12
- `buffa`: 0.9

## Binary Output

Build artifacts: `target/wasm32-unknown-unknown/release/polymarket_exchange.wasm`
