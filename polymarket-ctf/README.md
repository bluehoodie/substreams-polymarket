# Polymarket CTF Substreams Package

Substreams package for extracting events from the Polymarket Conditional Tokens Framework (CTF) contract on Polygon.

## Contract Information

- **Address**: `0x4D97DCd97eC945f40cF65F87097ACe5EA0476045`
- **Network**: Polygon
- **Explorer**: [View on Polygonscan](https://polygonscan.com/address/0x4D97DCd97eC945f40cF65F87097ACe5EA0476045)

## Available Modules

| Module | Description | Output Type |
|--------|-------------|-------------|
| `map_ctf_events` | Extracts CTF-specific events | `proto:ctf.v1.CtfEvents` |
| `map_erc1155_events` | Extracts ERC-1155 token events | `proto:ctf.v1.Erc1155Events` |
| `map_all_events` | Extracts all CTF and ERC-1155 events | `proto:ctf.v1.AllEvents` |

## Quick Start

### Build the WASM binary:

```bash
make build-ctf
# or
cd polymarket-ctf && substreams build
```

### Create the Substreams package:

```bash
make package-ctf
# or
cd polymarket-ctf && substreams pack
```

### Run the Substreams:

```bash
make run-ctf
# or
substreams run substreams.yaml map_all_events \
  --network polygon \
  --start-block -1000
```

## Event Types

The package extracts the following event categories:

### CTF Events
- `ConditionPreparation` - New prediction market condition prepared
- `ConditionResolution` - Condition resolved with payout numerators
- `PositionSplit` - Collateral split into conditional positions
- `PositionsMerge` - Conditional positions merged back to collateral
- `PayoutRedemption` - Winning positions redeemed for collateral

### ERC-1155 Events
- `TransferBatch` - Batch transfer of tokens
- `TransferSingle` - Single token transfer
- `ApprovalForAll` - Approval for all operator

## Dependencies

- `substreams`: ^0.7
- `substreams-ethereum`: ^0.11
- `ethabi`: ^18

## Binary Output

Build artifacts: `target/wasm32-unknown-unknown/release/polymarket_ctf.wasm`
