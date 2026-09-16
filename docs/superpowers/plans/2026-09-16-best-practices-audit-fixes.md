# Polymarket Substreams — Best-Practices Audit Fixes Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Close every actionable finding from the 2026-09-16 best-practices audit so the 8-package Polymarket Substreams workspace is clean for a public conference/website showcase.

**Architecture:** Surgical fixes across the existing Cargo workspace — one latent-panic fix (new shared helper), a DRY ABI de-duplication (6 packages reuse `polymarket-substreams-common::abi` instead of their own `build.rs`/`abi/`), manifest `doc:` strings, and README corrections. Ends with a lockstep `v0.13.0` bump committed on `fix/audit-v0.13.0`, **no tag** (owner pushes the tag).

**Tech Stack:** Rust (wasm32), Substreams (`substreams build`/`buf generate`), `polymarket-substreams-common` shared crate.

**Spec:** The audit findings are inline in each task below (no separate spec doc). Authority order for conflicts: the repo `CLAUDE.md` conventions > this plan.

## Global Constraints

- **No `.unwrap()`/`.expect()`/`panic!` in handler code** (non-test). `.expect()` is allowed only inside `#[cfg(test)]`. This is the invariant Task 1 restores.
- **Never hand-edit generated code** (`src/pb/`, `src/abi/`). Change the `.proto`/ABI JSON or the `use` path; regenerate via `substreams build`.
- **Address filtering uses 20-byte `[u8;20]` constants compared directly** — never hex strings. (Already true everywhere; do not regress.)
- **All 8 packages release in lockstep at one version.** Current: `v0.12.0`. Target after this work: `v0.13.0`.
- **Branch:** all work on `fix/audit-v0.13.0` (never commit to `main`). `fix/best-practices-audit` is the stale v0.12.0 round — do not reuse it.
- **Green gate for every task:** `cargo clippy --workspace --all-targets -- -D warnings` clean AND `cargo test --workspace` = 78+ pass / 0 fail. Package-level changes additionally require `cd <pkg> && substreams build` to succeed.
- **Do NOT push a git tag and do NOT publish.** The plan stops at a PR-ready branch.
- **Baseline (must hold before and after):** clippy clean; `cargo test --workspace` 78 passed / 0 failed.

## Rulings already made (owner-confirmed or defaulted — redirect if wrong)

- **R1 (owner):** Release handling = **stop before tag**. Final task bumps to `v0.13.0`, commits, opens a PR. No tag, no merge, no publish.
- **R2 (owner):** ethabi finding = **fix the README to `17`** (code works on 17; the README is wrong). No dependency bump.
- **R3 (default):** Finding #4 `CollateralOnramp` = **drop the phantom README row** (it is indexed by no module). Do not implement the contract — that is scope creep. Also fill the two `—` deploy-block cells with `4023686`.
- **R4 (default):** Finding #7 proto DX = **non-breaking only**. Document the field-encoding convention (addresses = `0x`-hex strings; ids/hashes = raw bytes) in the root README. Do **not** change any `.proto` field types (breaking for existing `v0.12.0` consumers) and do **not** do the cosmetic `tx` tag-number / `format_address`-naming churn.
- **R5 (default):** Finding #6 initialBlock accuracy = **pre-release validation step, not a code change** unless a discrepancy is found (needs chain access / `substreams auth`). Task 5 documents the check; if auth is unavailable it is deferred with a ledger note.
- **R6 (scope):** ABI de-dup (#2) covers **only the 6 packages whose contracts live in common** (collateral, ctf, exchange, neg-risk-adapter, neg-risk-ctf, wallet-factory). **Resolution is excluded** — its UMA/adapter ABIs are not in common. trader-index is already migrated.

## Model / role assignment (per repo CLAUDE.md)

- **Implementers:** Sonnet (all tasks are mechanical or well-specified).
- **Task reviewers & final whole-branch review:** Opus.
- Fix-loop escalation (rounds 4-5): bump implementer one tier above Sonnet.

## File Structure / blast radius

| Task | Files touched |
|---|---|
| 1 | `polymarket-substreams-common/src/lib.rs` (+helper +tests), `polymarket-resolution/src/lib.rs` (2 call sites + import) |
| 2 | 7 `substreams.yaml` (all except trader-index): add `doc:` per module |
| 3 | `README.md` (root), `polymarket-resolution/README.md` |
| 4 | 6 packages: delete `build.rs`, `abi/`, `src/abi/`, `pub mod abi;`; rewrite `use`/paths to `polymarket_substreams_common::abi::*`; trim `Cargo.toml` build-deps |
| 5 | (validation only — no code unless discrepancy found) |
| 6 | All 8 `substreams.yaml` (+ any versioned `Cargo.toml`/README spkg refs): `v0.12.0` → `v0.13.0` |

---

### Task 1: Fix the latent `to_u64()` handler panic (finding #1)

**Files:**
- Modify: `polymarket-substreams-common/src/lib.rs` (add `bigint_to_u64` after `bigint_to_u32`; add tests)
- Modify: `polymarket-resolution/src/lib.rs:9` (import), `:85`, `:143` (call sites)
- Test: `polymarket-substreams-common/src/lib.rs` `#[cfg(test)] mod tests`

**Interfaces:**
- Produces: `pub fn bigint_to_u64(bigint: &BigInt) -> u64` in `polymarket_substreams_common`.

**Why:** `BigInt::to_u64()` panics above `u64::MAX` (confirmed in `substreams-rs/.../scalar.rs:690`). These are ABI `uint256` fields → the only violation of the no-panic-in-handlers rule. Mirror the existing saturating `bigint_to_u32`.

- [ ] **Step 1: Write the failing tests** — add to `mod tests` in common:

```rust
#[test]
fn test_bigint_to_u64_normal() {
    assert_eq!(bigint_to_u64(&BigInt::from(12345u64)), 12345u64);
}

#[test]
fn test_bigint_to_u64_max() {
    assert_eq!(bigint_to_u64(&BigInt::from(u64::MAX)), u64::MAX);
}

#[test]
fn test_bigint_to_u64_overflow_saturates_to_zero() {
    use std::str::FromStr;
    // 2^64 — beyond u64::MAX. Must saturate to 0, never panic.
    let big = BigInt::from_str("18446744073709551616").expect("valid bigint literal");
    assert_eq!(bigint_to_u64(&big), 0);
}
```

- [ ] **Step 2: Run to verify failure**

Run: `cargo test -p polymarket-substreams-common bigint_to_u64`
Expected: FAIL (`bigint_to_u64` not found). If `BigInt::from_str` does not resolve, use the crate's decimal constructor (`BigInt::try_from("18446744073709551616")`) — pick whichever compiles; the test intent (2^64 → 0) is what matters.

- [ ] **Step 3: Add the helper** — after `bigint_to_u32` in `polymarket-substreams-common/src/lib.rs`:

```rust
#[inline]
pub fn bigint_to_u64(bigint: &BigInt) -> u64 {
    // `BigInt::to_u64()` panics on out-of-range values, so parse the decimal
    // string instead: any value that doesn't fit a u64 (or is negative) yields
    // 0 via `unwrap_or`, which cannot panic in a handler.
    bigint.to_string().parse::<u64>().unwrap_or(0)
}
```

- [ ] **Step 4: Swap the two resolution call sites**

- `polymarket-resolution/src/lib.rs:9` — add `bigint_to_u64` to the import:
  `use polymarket_substreams_common::{bigint_to_string, bigint_to_u64, build_tx_context, format_address};`
- `:85` — `expiration: e.expiration_timestamp.to_u64(),` → `expiration: bigint_to_u64(&e.expiration_timestamp),`
- `:143` — `expiration: e.expiration_time.to_u64(),` → `expiration: bigint_to_u64(&e.expiration_time),`

- [ ] **Step 5: Verify tests pass + invariant restored**

Run: `cargo test -p polymarket-substreams-common && cargo test -p polymarket-resolution`
Expected: PASS.
Then run this grep — **must print nothing** (no raw `to_u64` in non-test handler code):
`for f in $(find polymarket-*/src -name lib.rs); do awk '/#\[cfg\(test\)\]/{t=1} t==0 && /\.to_(u64|i64)\(\)/{print FILENAME":"NR}' "$f"; done`

- [ ] **Step 6: Full green gate**

Run: `cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace`
Expected: clippy clean; 81 passed / 0 failed (78 + 3 new).

- [ ] **Step 7: Commit**

```bash
git add polymarket-substreams-common/src/lib.rs polymarket-resolution/src/lib.rs
git commit -m "Add saturating bigint_to_u64 and remove to_u64 panic paths in resolution"
```

---

### Task 2: Add module `doc:` strings to 7 packages (finding #3)

**Files:** Modify `substreams.yaml` in: `polymarket-collateral`, `polymarket-ctf`, `polymarket-exchange`, `polymarket-neg-risk-adapter`, `polymarket-neg-risk-ctf`, `polymarket-resolution`, `polymarket-wallet-factory`. (trader-index already has them — do not touch.)

**Why:** `substreams info`/registry surfaces module `doc:` to consumers. 24 of 27 modules have none. For a public showcase this is discoverability polish.

**Source of truth for wording:** each package's own `README.md` Modules table already describes every module — transcribe those one-liners. Do not invent behavior; if a module isn't described in the README, describe exactly what its handler does (read the handler).

- [ ] **Step 1:** For each module in each of the 7 manifests, add a `doc:` field (block scalar `doc: |` or single-line) — one to three lines, mirroring the README description. Follow trader-index's existing `doc:` style for indentation. Example shape (collateral `map_all_events`):

```yaml
    output:
      type: proto:polymarket.collateral.v1.AllEvents
    doc: |
      All collateral events (pUSD token + CTF and Neg-Risk-CTF collateral adapters)
      decoded from a single pass, for the union of the three contract addresses.
```

- [ ] **Step 2: Verify every manifest still parses / packs**

Run per changed package: `cd <pkg> && substreams info` (or `substreams build`)
Expected: no YAML/manifest errors; `doc` shows on each module.

- [ ] **Step 3: Green gate** (docs-only, but confirm nothing broke)

Run: `cargo test --workspace`
Expected: unchanged pass count.

- [ ] **Step 4: Commit**

```bash
git add polymarket-collateral/substreams.yaml polymarket-ctf/substreams.yaml polymarket-exchange/substreams.yaml polymarket-neg-risk-adapter/substreams.yaml polymarket-neg-risk-ctf/substreams.yaml polymarket-resolution/substreams.yaml polymarket-wallet-factory/substreams.yaml
git commit -m "Add module doc strings to all remaining package manifests"
```

---

### Task 3: README corrections (findings #4, #5, #7-doc)

**Files:** Modify `README.md` (root), `polymarket-resolution/README.md`.

**Why / exact edits:**
1. **#4** — In the root README "Collateral Package Addresses" table, **delete the `CollateralOnramp 0x93070a847efEf7F70739046A929D47a521F5B8ee` row** (indexed by no module — verified). In the same table, replace the two `—` "Deploy Block" cells (for `ctf` and `trader-index`) with `4023686`.
2. **#5** — In `polymarket-resolution/README.md:77`, change `ethabi: ^18` to `ethabi: 17` (matches the pinned dependency; code builds and all tests pass on 17).
3. **#7 (non-breaking doc, per R4)** — Add a short "Field encoding" note to the root README stating the convention: **addresses are emitted as `0x`-prefixed lowercase hex strings; ids/hashes (`condition_id`, `question_id`, `order_hash`, etc.) are emitted as raw `bytes`** — so SQL/consumers hex-encode the bytes fields themselves.

- [ ] **Step 1:** Make the three edits above. Do not touch any `.proto` file.
- [ ] **Step 2: Verify** `CollateralOnramp` and `^18` no longer appear:

Run: `grep -rn 'CollateralOnramp\|ethabi.*18' README.md polymarket-resolution/README.md || echo "clean"`
Expected: `clean`.

- [ ] **Step 3: Commit**

```bash
git add README.md polymarket-resolution/README.md
git commit -m "Fix README: drop phantom CollateralOnramp, correct ethabi to 17, document field encoding"
```

---

### Task 4: De-duplicate ABIs — 6 packages reuse `polymarket-substreams-common::abi` (finding #2)

**Files (per package, for all 6):** `polymarket-collateral`, `polymarket-ctf`, `polymarket-exchange`, `polymarket-neg-risk-adapter`, `polymarket-neg-risk-ctf`, `polymarket-wallet-factory`:
- Delete: `build.rs`, `abi/` (the JSON dir), `src/abi/` (generated bindings)
- Modify: `src/lib.rs` — remove `pub mod abi;`; change `abi::<name>` / `use abi::<name>` → `polymarket_substreams_common::abi::<name>`
- Modify: `Cargo.toml` — remove the `build.rs`-only build-dependencies (the `substreams-ethereum` abigen build-dep and `ethabi` if it was build-only). Keep all runtime deps (`substreams`, `substreams-ethereum` runtime, `polymarket-substreams-common`, `hex-literal`, etc.).

**DO NOT TOUCH `polymarket-resolution`** (its `optimistic_oracle_v2`, `optimistic_oracle_v3`, `uma_ctf_adapter` ABIs are not in common) **or `polymarket-trader-index`** (already migrated — use it as the reference pattern for what a migrated package looks like: no `build.rs`, no `abi/`, decodes via `polymarket_substreams_common::abi::*`).

**Why:** common already generates all 8 of these contract ABIs under identical module names (`ctf_exchange`, `conditional_tokens`, `neg_risk_adapter`, `neg_risk_ctf`, `p_usd`, `ctf_collateral_adapter`, `neg_risk_ctf_collateral_adapter`, `deposit_wallet_factory`). The 6 packages ship byte-identical private copies + their own `build.rs`. Reuse removes 6 `build.rs` + 8 duplicate JSONs and the silent-drift risk. **Module names already match**, so the runtime change is a `use`-path swap only.

**Migration is mechanical and same-shape across all 6 — one implementer, one reviewable diff.** Do the packages one at a time within the task, verifying each builds before moving on.

- [ ] **Step 1 (per package):** delete `build.rs`, `abi/`, `src/abi/`; remove `pub mod abi;` from `src/lib.rs`.
- [ ] **Step 2 (per package):** repoint every `abi::` reference to `polymarket_substreams_common::abi::`. (e.g. `use abi::ctf_exchange::events as ex;` → `use polymarket_substreams_common::abi::ctf_exchange::events as ex;`.) The event/field names are unchanged.
- [ ] **Step 3 (per package):** trim `Cargo.toml` build-deps and any `build = "build.rs"` key. Confirm `polymarket-substreams-common` is a normal dependency (it already is).
- [ ] **Step 4 (per package): build the wasm + run its tests** — this is the safety net that proves the ABIs are equivalent:

Run: `cd <pkg> && substreams build && cargo test -p <pkg>`
Expected: builds clean; that package's existing tests pass **unchanged** (same decode behavior).

- [ ] **Step 5: Full workspace green gate**

Run: `cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace`
Expected: clippy clean; full suite passes.

- [ ] **Step 6: Confirm the duplication is gone**

Run: `for d in polymarket-collateral polymarket-ctf polymarket-exchange polymarket-neg-risk-adapter polymarket-neg-risk-ctf polymarket-wallet-factory; do ls "$d/build.rs" "$d/abi" 2>/dev/null && echo "LEFTOVER in $d"; done; echo done`
Expected: no `LEFTOVER` lines.

- [ ] **Step 7: Commit**

```bash
git add -A -- polymarket-collateral polymarket-ctf polymarket-exchange polymarket-neg-risk-adapter polymarket-neg-risk-ctf polymarket-wallet-factory
git commit -m "Reuse shared common::abi across 6 packages; drop duplicate build.rs and ABI JSON"
```

> **Note (owner):** Task 4 is the largest change and the one most worth splitting into its own PR if you want to de-risk the conference. It is self-contained — the plan can ship Tasks 1-3 + 6 first and hold Task 4 for a follow-up. Flag to the controller if you want that split.

---

### Task 5: Verify grouped-package `initialBlock` accuracy (finding #6) — validation, chain access required

**Files:** none unless a discrepancy is found (then the relevant `substreams.yaml` `initialBlock`).

**Why:** `polymarket-collateral` (3 contracts @ `84902320`) and `polymarket-resolution` (4 contracts @ `29786052`) each use one `initialBlock`. That is correct only if the pin is ≤ the earliest contract's deploy block; a contract deployed earlier would silently miss events.

- [ ] **Step 1:** Confirm `substreams auth` is available. **If not, STOP this task and record in the ledger: "Task 5 deferred — no chain auth; owner to verify deploy blocks during pre-release validation."** (Per R5 this does not block the branch.)
- [ ] **Step 2 (if authed):** Use the `running-substreams-validation` skill. For each grouped contract, confirm its first event is at or after the pinned `initialBlock` (query a range starting a few blocks before the pin and confirm no events precede it). Endpoint `polygon.streamingfast.io:443`.
- [ ] **Step 3:** If any contract predates its pin, lower that manifest's `initialBlock` to the earliest deploy block among the group, rebuild, commit. Otherwise record "Task 5: verified, pins correct" in the ledger — no code change.

---

### Task 6: Lockstep version bump to `v0.13.0`, commit, open PR (R1 — stop before tag)

**Files:** all 8 `substreams.yaml` `package.version`, plus any versioned references the release skill enumerates (`Cargo.toml` package versions, README `.spkg` paths, `publish-all.sh`).

**Why:** all 8 packages release in lockstep; this work is a release's worth of changes. Owner chose stop-before-tag.

- [ ] **Step 1:** Use the `releasing-substreams-packages` skill to bump every manifest (and every other version reference it lists) from `v0.12.0` → `v0.13.0`. **Do the bump + single commit ONLY. Do NOT create the tag and do NOT push a tag.**
- [ ] **Step 2: Final green gate**

Run: `cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace`
Expected: clippy clean; all tests pass.

- [ ] **Step 3: Commit the bump**

```bash
git add -A
git commit -m "Bump all Polymarket Substreams packages to v0.13.0"
```

- [ ] **Step 4: Open the PR** (no merge, no tag) via superpowers:finishing-a-development-branch — target `main`, summarize the audit fixes. Hand the PR URL back to the owner for tag/publish.

---

## Self-review (author checklist — done)

- **Spec coverage:** findings #1→T1, #2→T4, #3→T2, #4/#5/#7→T3, #6→T5, release→T6. All 7 findings + release mapped. ✅
- **Placeholder scan:** exact code for T1; exact edits for T3; T2/T4 are mechanical same-shape transforms with per-item acceptance greps/builds (no fabricated per-package final source, deliberately — the wasm-build + unchanged-tests gate is the correctness proof). ✅
- **Type consistency:** new symbol is `bigint_to_u64` everywhere; ABI module names verified identical between common and the 6 packages. ✅
- **Ordering:** T1 (helper) before nothing depends on it; T4 (biggest/riskiest) isolated and splittable; T6 (bump) last. ✅
