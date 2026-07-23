# Polymarket Substreams Packages

A collection of Substreams packages for extracting and processing events from Polymarket smart contracts on the Polygon network.

## Overview

| Package | Contract | Address | Deploy Block |
|---------|----------|---------|--------------|
| [polymarket-ctf](./polymarket-ctf) | Conditional Tokens Framework | `0x4D97DCd97eC945f40cF65F87097ACe5EA0476045` | — |
| [polymarket-exchange](./polymarket-exchange) | CTF Exchange V2 | `0xE111180000d2663C0091e4f400237545B87B996B` | 84902353 |
| [polymarket-neg-risk-ctf](./polymarket-neg-risk-ctf) | Neg Risk CTF Exchange V2 | `0xe2222d279d744050d28e00520010520000310F59` | 85058176 |
| [polymarket-neg-risk-adapter](./polymarket-neg-risk-adapter) | NegRiskAdapter | `0xd91E80cF2E7be2e162c6513ceD06f1dD0dA35296` | 50505403 |
| [polymarket-collateral](./polymarket-collateral) | pUSD + Collateral Adapters | Multiple (see below) | 84902320 |
| [polymarket-resolution](./polymarket-resolution) | UMA Oracle + CTF Adapter | Multiple (see below) | 29786052 |
| [polymarket-wallet-factory](./polymarket-wallet-factory) | DepositWalletFactory | `0x00000000000Fb5C9ADea0298D729A0CB3823Cc07` | 84902000 |
| [polymarket-trader-index](./polymarket-trader-index) | All contracts (foundational user-activity index) | — | 4023686 |

### Collateral Package Addresses

| Contract | Address |
|----------|---------|
| pUSD | `0xC011a7E12a19f7B1f670d46F03B03f3342E82DFB` |
| CollateralOnramp | `0x93070a847efEf7F70739046A929D47a521F5B8ee` |
| CtfCollateralAdapter | `0xAdA100Db00Ca00073811820692005400218FcE1f` |
| NegRiskCtfCollateralAdapter | `0xadA2005600Dec949baf300f4C6120000bDB6eAab` |

### Resolution Package Addresses

| Contract | Address |
|----------|---------|
| UMA Oracle V2 | `0xee3afe347D5C74317041E2618C49534dAf887c24` |
| UMA Oracle V3 | `0x5953f2538F613E05bAED8A5AeFa8e6622467AD3D` |
| CTF Adapter V2 | `0x6A9D222616C90FcA5754cd1333cFD9b7fb6a4F74` |
| CTF Adapter V3 | `0x2f5e3684cb1F318eC51b00eDba38D79ac2c0aA9D` |

## Architecture

All packages follow a unified design pattern:

- **Single-pass event extraction** — all events extracted in one block traversal
- **Byte-level address comparison** — 20-byte array for fast contract filtering (~25x faster than string)
- **Consistent protobuf schema** — all events include `TransactionContext` metadata
- **Modular outputs** — separate modules for different event categories

## Block Indexes & Filtering

Every package uses **block indexes** so the Substreams engine can *skip blocks it never needs to read*. Polymarket activity is sparse relative to all of Polygon, so this is the biggest lever on backfill speed and cost: a contract active in 0.5% of blocks costs roughly 0.5% as much to stream, and filtering by a single wallet is far more selective still.

A block index emits a set of string **keys** per block (`sf.substreams.index.v1.Keys`). A downstream `map`/`store` module declares a `blockFilter` with a query over those keys; **blocks whose keys don't match are never read, decoded, or processed**.

Two kinds of index are in play:

- **Contract-level skipping** (`evt_addr:`) is delegated to the shared **foundational `ethereum_common` index** (`eth_common:index_events`), imported by every package. It emits `evt_addr:`/`evt_sig:` keys for every block, is computed once and reused across *all* Substreams, and is never invalidated by our releases — so there is no per-package index to hand-roll or re-warm. We don't reinvent it.
- **Data-derived skipping** (`user:`) needs keys decoded from event *payloads* — *which wallet* acted, not just *which contract* — something the foundational `evt_addr`/`evt_sig` index cannot produce. This is the one genuinely-custom index we keep local: **`polymarket-trader-index`'s `index_users`**, the canonical way to filter by wallet across *all* Polymarket contracts.

### Key namespaces

| Key | Emitted by | Meaning |
|-----|-----------|---------|
| `evt_addr:<address>` | foundational `ethereum_common` — `eth_common:index_events` (imported by every package) | the block contains a log from this targeted contract |
| `user:<address>` | `polymarket-trader-index` — `index_users` | this wallet took **any** action on Polymarket in the block (trades, splits/merges, redemptions, conversions, pUSD transfers, ERC-1155 transfers, wallet deploys), across **all** contracts |

All keys are lowercase hex with a `0x` prefix.

### Query syntax (SQE)

A `blockFilter` query is a boolean expression over keys — `&&` (and), `||` (or), `-` (not), `( )` grouping. It can be hard-coded in the manifest (`query.string:`) or supplied at run time (`query.params: true`, read from the module's `params` input).

> The index alone changes nothing — a module skips blocks **only** when it declares a `blockFilter`. The query namespace must match the emitted keys exactly (`evt_addr:` vs `user:`); a mismatch silently matches no blocks.

### Example use-cases

**1. Stream a single contract's events.** Every `map_*` module is already wired to the foundational `eth_common:index_events` (`evt_addr:`), so backfills skip irrelevant blocks automatically — with no local index to warm:

```bash
substreams run polymarket-collateral/substreams.yaml map_pusd_events -s 85049190 -t +500000
# only blocks containing a pUSD log are processed
```

**2. Track one (or many) wallets' full Polymarket activity.** This is the job of `polymarket-trader-index`: its `index_users` blockIndex emits a `user:<address>` key for every wallet that took *any* action — on *any* Polymarket contract — in a block, from CTF genesis onward. Because a given wallet appears in only a tiny fraction of Polygon's ~88M blocks, filtering on `user:` lets a stream read **only** that wallet's active blocks.

The powerful pattern is **composition**: use `index_users` as the block-skip *gate*, and the per-contract packages as the *decoders*. Build a small downstream package that imports both, and the engine skips every block the wallet was inactive in while you reuse the already-decoded event structs from each contract package:

```yaml
imports:
  pmusers:   ../polymarket-trader-index/polymarket-trader-index-v0.11.0.spkg
  exchange:  ../polymarket-exchange/polymarket-exchange-v0.11.0.spkg
  ctf:       ../polymarket-ctf/polymarket-ctf-v0.11.0.spkg
  # …import whichever contract packages you care about

modules:
  - name: map_wallet_activity
    kind: map
    blockFilter:
      module: pmusers:index_users        # the user: gate — skips blocks the wallet never touched
      query: { params: true }            # e.g. "user:0xA || user:0xB || user:0xC"
    inputs:
      - params: string
      - map: exchange:map_all_events     # already evt_addr-filtered + decoded for you
      - map: ctf:map_all_events
    output:
      type: proto:my.types.WalletActivity
```
```rust
// Your handler runs ONLY on the wallet's active blocks. For each, the per-contract
// maps hand you fully-decoded events (already evt_addr-filtered upstream) — keep the
// ones belonging to the watched wallet(s) and stitch them into one record.
#[substreams::handlers::map]
fn map_wallet_activity(
    params: String,
    exchange: exchange_pb::AllEvents,
    ctf: ctf_pb::AllEvents,
) -> Result<WalletActivity, Error> { /* filter to params' wallets, merge */ }
```

How the two indexes cooperate: the downstream `blockFilter` on `index_users` decides *which blocks run at all* (the wallet's blocks); each upstream `map_all_events` carries its own `eth_common:index_events` (`evt_addr:`) filter, so within a surviving block only the contracts the wallet actually touched produce events. You pay only for the wallet's blocks, and only decode the contracts you imported.

Run it with an `||` list of wallets (single or many — the engine skips every block none of them touched):

```bash
substreams run my-package/substreams.yaml map_wallet_activity \
  -p map_wallet_activity="user:0xabc… || user:0xdef… || user:0x123…" \
  -s 4023686 -t +88000000 --production-mode
```

> **Warm the index first.** `index_users` is versioned, so the first `--production-mode` full-range pass *computes* the index (and returns nothing). Once warm, filtered runs skip straight to the wallet's active blocks. See the `warming-substreams-indexes` workflow.

**3. Compute a trader's running PnL.** A trader's PnL is a reduction over their collateral cash-flows and outcome-token positions — and every input you need is already decoded by these packages. Gate on `user:` so you read only the trader's blocks, then fold the relevant events into a `store`. The events and their PnL effect:

| Event — package · map | PnL effect |
|---|---|
| `OrderFilled` / `OrdersMatched` — `exchange`, `neg-risk-ctf` · `map_all_events` | a buy spends collateral and adds outcome tokens; a sell is the reverse. The cash-flow and cost basis per `token_id`, plus the **last traded price** used to mark open positions. |
| `PositionSplit` / `PositionsMerge` — `ctf` · `map_all_events` | a split locks *N* collateral → *N* of **each** outcome token (cash out, tokens in); a merge is the reverse. Moves position and cash with no market price. |
| `PayoutRedemption` — `ctf` · `map_all_events` | after resolution, winning tokens burn for collateral — **realizes** that position at its payout. |
| `ConditionResolution` — `resolution` · `map_resolution_events` | sets each outcome's final payout (0 or 1) — the **mark price** for valuing any still-open position. |

Wire one `store` keyed by the trader, gated by the user index, fed the decoded events. Accumulate **realized cash** and **net position per `token_id`** (both additive); a small downstream `map` then reads the store and emits the PnL number = `realized_cash + Σ(position[token_id] × mark[token_id])`:

```yaml
imports:
  pmusers:    ../polymarket-trader-index/polymarket-trader-index-v0.11.0.spkg
  exchange:   ../polymarket-exchange/polymarket-exchange-v0.11.0.spkg
  negctf:     ../polymarket-neg-risk-ctf/polymarket-neg-risk-ctf-v0.11.0.spkg
  ctf:        ../polymarket-ctf/polymarket-ctf-v0.11.0.spkg
  resolution: ../polymarket-resolution/polymarket-resolution-v0.11.0.spkg

modules:
  - name: store_trader_pnl
    kind: store
    updatePolicy: add
    valueType: bigdecimal
    blockFilter:
      module: pmusers:index_users        # only the trader's blocks run
      query: { params: true }            # "user:0x<trader>"
    inputs:
      - params: string
      - map: exchange:map_all_events
      - map: negctf:map_all_events
      - map: ctf:map_all_events
      - map: resolution:map_resolution_events
```
```rust
// Runs ONLY on the trader's active blocks. Keep events where the trader is the
// maker/taker (fills) or the stakeholder (split/merge/redeem); ignore the rest.
#[substreams::handlers::store]
fn store_trader_pnl(
    params: String,
    exchange: exchange_pb::AllEvents,
    negctf:   negctf_pb::AllEvents,
    ctf:      ctf_pb::AllEvents,
    resolution: resolution_pb::ResolutionEvents,
    store: StoreAddBigDecimal,
) {
    let trader = parse_user(&params);
    // For each kept event, accumulate two additive ledgers:
    //   cash:  +collateral received (sells, merges, redemptions)
    //          −collateral paid     (buys, splits)
    //   pos:   net outcome tokens held, per token_id
    // store.add(format!("cash:{trader}"), cash_delta);
    // store.add(format!("pos:{trader}:{token_id}"), qty_delta);
    // Track each token_id's mark: last fill price, or its ConditionResolution payout once resolved.
    // store.set(...) the mark in a sibling `set`-policy store, then a downstream map computes:
    //   pnl = cash + Σ(pos[token_id] × mark[token_id])
}
```

Because `store_trader_pnl` runs only on the wallet's active blocks — typically a few hundred of Polygon's ~88M — a full-history PnL backfill reads a vanishingly small slice of the chain, once `index_users` is warm. Swap the single `user:0x…` for an `||` list to track a whole cohort's PnL in one stream.

## Quick Start

### Prerequisites

- Rust with `wasm32-unknown-unknown` target
- Substreams CLI (`substreams` command)
- `buf` CLI (required for protobuf generation)
- Substreams API key — authenticate with `substreams auth` ([get a key at thegraph.market](https://thegraph.market))

### Build

```bash
# Build all packages
make build-all

# Build individual packages
make build-exchange
make build-ctf
make build-neg-risk-ctf
make build-neg-risk-adapter
make build-collateral
make build-resolution
make build-wallet-factory
make build-trader-index
```

### Run

```bash
# V2 exchange events (from V2 deploy block)
substreams run polymarket-exchange/substreams.yaml map_all_events \
  -s 84902353 -t +1000

# NegRisk adapter market events
substreams run polymarket-neg-risk-adapter/substreams.yaml map_market_events \
  -s 50505403 -t +10000

# pUSD wrap/unwrap events
substreams run polymarket-collateral/substreams.yaml map_pusd_events \
  -s 84902320 -t +10000

# Resolution pipeline (oracle + adapter events)
substreams run polymarket-resolution/substreams.yaml map_resolution_events \
  -s 29786052 -t +10000

# Wallet deployments
substreams run polymarket-wallet-factory/substreams.yaml map_factory_events \
  -s 84902000 -t +10000

# One wallet's full Polymarket activity (user: index — pass the wallet as a param)
substreams run polymarket-trader-index/substreams.yaml map_user_activity \
  -p map_user_activity="user:0xefe62ed1df3f4afcc8a2afe2e4e8fcde965c9520" \
  -s 4023686 -t +1000000
```

## V2 Event Changes (exchange packages)

Polymarket launched V2 contracts in April 2026 with a redesigned order struct:

**`OrderFilled`**: `side` (uint8) + `token_id` replace the old `maker_asset_id`/`taker_asset_id`. New fields: `builder`, `metadata`.

**`OrdersMatched`**: Same restructuring — `side` (uint8) + `token_id` added, dual asset IDs removed.

**`FeeCharged`**: Removed `tokenId` parameter. V2 signature is `FeeCharged(address indexed recipient, uint256 amount)`.

**Removed**: `OrderCancelled`, `TokenRegistered` (V2 no longer emits these).

**New events**: `UserPaused`/`UserUnpaused`, `OrderPreapproved`/`OrderPreapprovalInvalidated`, `FeeReceiverUpdated`, `MaxFeeRateUpdated`, admin events.

## Event Categories

### Exchange / Neg Risk CTF (V2)
- Trading: `OrderFilled` (with `side`, `token_id`), `OrdersMatched`
- Fee: `FeeCharged`, `FeeReceiverUpdated`, `MaxFeeRateUpdated`
- Admin: `NewAdmin`, `NewOperator`, `RemovedAdmin`, `RemovedOperator`
- Pause: `UserPaused`, `UserUnpaused`, `UserPauseBlockIntervalUpdated`
- Approval: `OrderPreapproved`, `OrderPreapprovalInvalidated`

### CTF (unchanged)
- `ConditionPreparation`, `ConditionResolution`
- `PositionSplit`, `PositionsMerge`, `PayoutRedemption`
- ERC-1155: `TransferSingle`, `TransferBatch`, `ApprovalForAll`

### NegRiskAdapter
- Market: `MarketPrepared`, `QuestionPrepared`, `OutcomeReported`
- Trading: `PositionSplit`, `PositionsMerge`, `PositionsConverted`, `PayoutRedemption`
- Admin: `NewAdmin`, `RemovedAdmin`

### Resolution (UMA Oracle + CTF Adapter)
- Oracle: `ResolutionProposal`, `ResolutionDispute`, `ResolutionSettlement` (unified across V2/V3)
- Adapter: `QuestionInitialized`, `QuestionResolved`, `QuestionEmergencyResolved`, `QuestionFlagged`/`QuestionUnflagged`, `QuestionPaused`/`QuestionUnpaused`, `QuestionReset`, `AncillaryDataUpdated`
- Admin: `NewAdmin`, `RemovedAdmin`
- Alerts: `DisputeAlert` (disputes, emergency resolutions, flags, pauses, resets)

### Collateral
- pUSD: `Transfer`, `Wrapped`, `Unwrapped`
- Adapters: `PositionSplit`, `PositionsMerged`, `PositionsRedeemed`
- NegRisk adapter only: `PositionsConverted`

### Wallet Factory
- `WalletDeployed`, `ImplementationAuthorized`

## Dependencies

- `substreams`: ^0.7
- `substreams-ethereum`: ^0.11
- `ethabi`: ^18

## Resources

- [Substreams Documentation](https://docs.streamingfast.io/substreams)
- [Polymarket](https://polymarket.com)
- [Polygon Network](https://polygon.technology)

## License

MIT
