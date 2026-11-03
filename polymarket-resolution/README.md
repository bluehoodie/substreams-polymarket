# Polymarket Resolution Substreams Package

Substreams package for the full Polymarket resolution pipeline — UMA Optimistic Oracle disputes, the UMA CTF Adapter question lifecycle, and Protocol V2 module resolutions — on Polygon.

## Contract Information

- **Network**: Polygon
- **Initial Block**: 29786052

| Contract | Address |
|----------|---------|
| UMA Oracle V2 | `0xee3afe347D5C74317041E2618C49534dAf887c24` |
| UMA Oracle V3 | `0x5953f2538F613E05bAED8A5AeFa8e6622467AD3D` |
| CTF Adapter V2 | `0x6A9D222616C90FcA5754cd1333cFD9b7fb6a4F74` |
| CTF Adapter V3 | `0x2f5e3684cb1F318eC51b00eDba38D79ac2c0aA9D` |
| Protocol V2 BinaryModule (proxy) | `0x1000008dD9001B968442c1000017eaE6E0dA00Ba` |
| Protocol V2 NegRiskModule (proxy) | `0x200000900045e3B6259600682756002200028933` |

## Available Modules

| Module | Description | Output Type |
|--------|-------------|-------------|
| `map_oracle_events` | Extracts UMA Oracle proposals, disputes, settlements (V2 + V3) | `proto:polymarket.resolution.v1.OracleEvents` |
| `map_adapter_events` | Extracts CTF Adapter question lifecycle events (V2 + V3) | `proto:polymarket.resolution.v1.AdapterEvents` |
| `map_dispute_alerts` | Extracts a unified alert stream (disputes, emergency resolutions, flags, pauses, resets) | `proto:polymarket.resolution.v1.DisputeAlerts` |
| `map_module_events` | Extracts Protocol V2 `ConditionResolved` (final payouts) from BinaryModule and NegRiskModule | `proto:polymarket.resolution.v1.ModuleEvents` |
| `map_resolution_events` | Extracts the full combined resolution pipeline (oracle, adapter, alerts, modules) | `proto:polymarket.resolution.v1.ResolutionEvents` |

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

### Module Events (Protocol V2 BinaryModule + NegRiskModule)
- `ModuleConditionResolved` - A condition was resolved with its final payout vector
  (`condition_id` as 0x hex of the bytes31 ConditionId, `payouts`, `module` = `binary` | `neg_risk`).
  Markets settled through Protocol V2 resolve here rather than through Conditional Tokens'
  `ConditionResolution`. These are settlements, not disputes, so they add no dispute alerts.

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

- `substreams`: ^0.8
- `substreams-ethereum`: ^0.12
- `buffa`: 0.9

## Binary Output

Build artifacts: `target/wasm32-unknown-unknown/release/polymarket_resolution.wasm`
