# Polymarket Collateral Substreams Package

Substreams package for extracting pUSD collateral-token and CTF collateral-adapter events from Polymarket on Polygon.

## Contract Information

- **Network**: Polygon
- **Initial Block**: 85049190

| Contract | Address |
|----------|---------|
| pUSD | `0xC011a7E12a19f7B1f670d46F03B03f3342E82DFB` |
| CtfCollateralAdapter | `0xAdA100Db00Ca00073811820692005400218FcE1f` |
| NegRiskCtfCollateralAdapter | `0xadA2005600Dec949baf300f4C6120000bDB6eAab` |

## Available Modules

| Module | Description | Output Type |
|--------|-------------|-------------|
| `map_pusd_events` | Extracts pUSD collateral-token events | `proto:polymarket.collateral.v1.PusdEvents` |
| `map_ctf_adapter_events` | Extracts CtfCollateralAdapter events | `proto:polymarket.collateral.v1.CtfAdapterEvents` |
| `map_neg_risk_ctf_adapter_events` | Extracts NegRiskCtfCollateralAdapter events | `proto:polymarket.collateral.v1.NegRiskCtfAdapterEvents` |
| `map_all_events` | Extracts all collateral events | `proto:polymarket.collateral.v1.AllEvents` |

## Quick Start

### Build the WASM binary:

```bash
make build-collateral
# or
cd polymarket-collateral && substreams build
```

### Create the Substreams package:

```bash
cd polymarket-collateral && substreams pack
```

### Run the Substreams:

```bash
substreams run substreams.yaml map_all_events \
  --network polygon \
  --start-block 85049190 \
  --stop-block +1000
```

## Event Types

The package extracts the following event categories:

### pUSD Events
- `Transfer` - pUSD token transfer
- `Wrapped` - Collateral wrapped into pUSD
- `Unwrapped` - pUSD unwrapped back to collateral

### CTF Collateral Adapter Events
- `PositionSplit` - Collateral split into conditional positions
- `PositionsMerged` - Conditional positions merged back to collateral
- `PositionsRedeemed` - Winning positions redeemed for collateral

### NegRisk CTF Collateral Adapter Events
- `PositionSplit` - Collateral split into conditional positions
- `PositionsMerged` - Conditional positions merged back to collateral
- `PositionsRedeemed` - Winning positions redeemed for collateral
- `PositionsConverted` - Neg-risk positions converted across outcomes

## Dependencies

- `substreams`: ^0.7
- `substreams-ethereum`: ^0.11
- `ethabi`: ^18

## Binary Output

Build artifacts: `target/wasm32-unknown-unknown/release/polymarket_collateral.wasm`
