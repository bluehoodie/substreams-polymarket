# Polymarket Neg Risk Adapter Substreams Package

Substreams package for extracting events from the Polymarket NegRiskAdapter contract on Polygon.

## Contract Information

- **Address**: `0xd91E80cF2E7be2e162c6513ceD06f1dD0dA35296`
- **Network**: Polygon
- **Initial Block**: 50505403
- **Explorer**: [View on Polygonscan](https://polygonscan.com/address/0xd91E80cF2E7be2e162c6513ceD06f1dD0dA35296)

## Available Modules

| Module | Description | Output Type |
|--------|-------------|-------------|
| `map_market_events` | Extracts market/question lifecycle events | `proto:polymarket.neg_risk_adapter.v1.MarketEvents` |
| `map_trading_events` | Extracts position and redemption events | `proto:polymarket.neg_risk_adapter.v1.TradingEvents` |
| `map_admin_events` | Extracts admin role events | `proto:polymarket.neg_risk_adapter.v1.AdminEvents` |
| `map_all_events` | Extracts all NegRiskAdapter events | `proto:polymarket.neg_risk_adapter.v1.AllEvents` |

## Quick Start

### Build the WASM binary:

```bash
make build-neg-risk-adapter
# or
cd polymarket-neg-risk-adapter && substreams build
```

### Create the Substreams package:

```bash
cd polymarket-neg-risk-adapter && substreams pack
```

### Run the Substreams:

```bash
substreams run substreams.yaml map_all_events \
  --network polygon \
  --start-block 50505403 \
  --stop-block +10000
```

## Event Types

The package extracts the following event categories:

### Market Events
- `MarketPrepared` - New neg-risk market prepared
- `QuestionPrepared` - Question added to a market
- `OutcomeReported` - Outcome reported for a question

### Trading Events
- `PositionSplit` - Collateral split into conditional positions
- `PositionsMerge` - Conditional positions merged back to collateral
- `PositionsConverted` - Neg-risk positions converted across outcomes
- `PayoutRedemption` - Winning positions redeemed for collateral

### Admin Events
- `NewAdmin` - New admin added
- `RemovedAdmin` - Admin removed

## Dependencies

- `substreams`: ^0.7
- `substreams-ethereum`: ^0.11
- `ethabi`: ^18

## Binary Output

Build artifacts: `target/wasm32-unknown-unknown/release/polymarket_neg_risk_adapter.wasm`
