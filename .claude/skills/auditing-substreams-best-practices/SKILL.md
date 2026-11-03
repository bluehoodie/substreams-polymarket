---
name: auditing-substreams-best-practices
description: Audit the polymarket-* Substreams packages against streamingfast/substreams-skills develop (weekly, report-only). Use for the weekly best-practices check or when asked whether the packages still follow upstream guidance.
---

# Auditing substreams best practices

Checks this repo's 8 `polymarket-*` packages (and `polymarket-substreams-common`) against
the upstream [substreams-skills](https://github.com/streamingfast/substreams-skills)
`develop` guidance: `skills/substreams-dev`, `skills/substreams-ethereum`,
`skills/substreams-testing`.

## Rules for the weekly check

- **Report-only.** Post findings for Colin's approval. Open no pull requests, push no
  branches, file no fix issues until Colin approves a finding.
- The audited upstream commit lives in [`docs/substreams-skills-baseline.md`](../../../docs/substreams-skills-baseline.md).
  Move it **only after Colin approves** the findings for the new commit, together with
  the matching "Last audited against commit" line in `CLAUDE.md`.
- If `develop` has no new commits touching the three skill directories since the
  baseline, report "no upstream changes" and stop.

## Procedure

1. Read the baseline commit from `docs/substreams-skills-baseline.md`.
2. Clone or fetch `streamingfast/substreams-skills` `develop`, then
   `git diff <baseline>..origin/develop -- skills/substreams-dev skills/substreams-ethereum skills/substreams-testing`.
3. For each changed piece of guidance, check the repo against it, using the checklist
   below as the map of where each rule lives. Skip checklist items the diff didn't touch.
4. Drop anything listed under **Accepted exceptions**.
5. Report: upstream commit range, each finding (rule, upstream file, our files, proposed
   change, breaking or not), and "no change needed" items you checked. Ask Colin which
   to act on and whether to move the baseline.

## Checklist

### Block filtering and indexes
- Every module that reads `sf.ethereum.type.v2.Block` declares a `blockFilter` on the
  foundational `eth_common:index_events` with an `evt_addr:` query for exactly the
  contracts it decodes. Query keys must match the index's namespace (`evt_addr:` vs `user:`).
- `initialBlock` is the contract's deploy block (earliest one for multi-contract modules).
- The `eth_common` import is pinned to a versioned spkg.io URL.

### Handlers
- First statement is `substreams::skip_empty_output();`.
- Block input is the lazy view `&eth::BlockLazyView<'_>`; logs are reached via
  `polymarket_substreams_common::block_logs(blk, is_x)?`, not `blk.logs()`/`transactions()`.
- Address predicates compare 20-byte constants, never hex strings.
- `map_all_events` (and `map_resolution_events`) classify in a single pass.
- No `.unwrap()` / `.expect()` / `panic!` outside `#[cfg(test)]`, benches and examples.
  Decode with `if let Ok(ev) = Event::decode(...)`.
- Shared helpers come from `polymarket-substreams-common`, not per-package copies.
- Every event carries `tx: Option<TransactionContext>`.

### Module layout
- substreams-dev Performance §1 prefers one extraction module per block. See the
  accepted exception for per-category modules below.

### Tests
- Handler tests call `substreams::testing::map!(map_x(blk))` through `with_lazy_block`,
  never `__impl_map_x` directly.
- Each package has a real Polygon block fixture under `src/testdata/` and a test that
  asserts its known events; `tests/assertions.yaml` drives `substreams run --test-file`.
- `cargo test --workspace` runs in `release.yml` on `v*` tags only (CI policy; don't
  propose moving it to push/PR).

### Build and manifest hygiene
- Release profile and wasm size: compare builds with `scripts/wasm_bench.js`, not native
  Criterion, before proposing profile changes.
- `src/pb/` and `src/abi/` are generated; findings target `.proto` / ABI JSON.
- All 8 manifests share one `package.version`; README `.spkg` references match.
- Each module has a `doc:`.

## Accepted exceptions

Don't report these unless upstream guidance about them changes materially; if it does,
say so and ask Colin whether the exception still holds.

- **trader-index `index_users` is a pass-through** of `map_user_keys`. A `blockIndex`
  module can't declare a `blockFilter`, so the work lives in `map_user_keys`.
- **trader-index takes `map_user_keys` and `index_users` from the published v1.0.0
  (`warm:` import).** Changing them re-hashes the index and forces a re-warm from block
  4023686; only a deliberate release does that.
- **Per-category modules stay** (`map_exchange_events`, `map_fee_events`,
  `map_admin_events`, `map_pause_events`, `map_approval_events`, `map_trading_events`,
  `map_market_events`, `map_pusd_events`, `map_ctf_adapter_events`,
  `map_neg_risk_ctf_adapter_events`, `map_oracle_events`, `map_adapter_events`,
  `map_dispute_alerts`, `map_ctf_events`, `map_erc1155_events`) alongside
  `map_all_events` / `map_resolution_events`. Colin decided to keep them as public API
  (SUB-4, 2026-11-03), despite the one-extraction-module guidance.
