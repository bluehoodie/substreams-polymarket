# Polymarket Resolution Substreams Package

Substreams package for the full Polymarket resolution pipeline — UMA Optimistic Oracle disputes plus the UMA CTF Adapter question lifecycle — on Polygon.

## Contract Information

- **Network**: Polygon
- **Initial Block**: 29786052

| Contract | Address |
|----------|---------|
| UMA Oracle V2 | `0xee3afe347D5C74317041E2618C49534dAf887c24` |
| UMA Oracle V3 | `0x5953f2538F613E05bAED8A5AeFa8e6622467AD3D` |
| CTF Adapter V2 | `0x6A9D222616C90FcA5754cd1333cFD9b7fb6a4F74` |
| CTF Adapter V3 | `0x2f5e3684cb1F318eC51b00eDba38D79ac2c0aA9D` |

## Available Modules

| Module | Description | Output Type |
|--------|-------------|-------------|
| `map_oracle_events` | Extracts UMA Oracle proposals, disputes, settlements (V2 + V3) | `proto:polymarket.resolution.v1.OracleEvents` |
| `map_adapter_events` | Extracts CTF Adapter question lifecycle events (V2 + V3) | `proto:polymarket.resolution.v1.AdapterEvents` |
| `map_dispute_alerts` | Extracts a unified alert stream (disputes, emergency resolutions, flags, pauses, resets) | `proto:polymarket.resolution.v1.DisputeAlerts` |
| `map_resolution_events` | Extracts the full combined resolution pipeline | `proto:polymarket.resolution.v1.ResolutionEvents` |

## Quick Start

### Build the WASM binary:

```bash
make build-resolution
# or
cd polymarket-resolution && substreams build
```

### Create the Substreams package:

```bash
cd polymarket-resolution && substreams pack
```

### Run the Substreams:

```bash
substreams run substreams.yaml map_resolution_events \
  --network polygon \
  --start-block 29786052 \
  --stop-block +10000
```

## Event Types

The package extracts the following event categories:

### Oracle Events (UMA Optimistic Oracle V2 + V3, unified)
- `ResolutionProposal` - A price/assertion was proposed (V2 `ProposePrice`, V3 `AssertionMade`)
- `ResolutionDispute` - A proposal was disputed (V2 `DisputePrice`, V3 `AssertionDisputed`)
- `ResolutionSettlement` - A proposal was settled (V2 `Settle`, V3 `AssertionSettled`)

### Adapter Events (UMA CTF Adapter V2 + V3)
- `QuestionInitialized` - Question registered with the adapter
- `QuestionResolved` - Question resolved with payouts
- `QuestionEmergencyResolved` - Question emergency-resolved
- `QuestionFlagged` / `QuestionUnflagged` - Question flagged / unflagged
- `QuestionPaused` / `QuestionUnpaused` - Question paused / unpaused
- `QuestionReset` - Question reset for re-resolution
- `AncillaryDataUpdated` - Question ancillary data updated
- Admin changes - `NewAdmin` / `RemovedAdmin`

### Dispute Alerts
- `DisputeAlert` - Unified alert emitted for disputes, emergency resolutions, flags, pauses, and resets across both oracle and adapter contracts

## Dependencies

- `substreams`: ^0.7
- `substreams-ethereum`: ^0.11
- `ethabi`: ^18

## Binary Output

Build artifacts: `target/wasm32-unknown-unknown/release/polymarket_resolution.wasm`
