# Polymarket V2 Migration Design

**Date:** 2026-05-19  
**Branch:** feature/polymarket-v2-migration

## Overview

Polymarket launched V2 smart contracts in late March/early April 2026. This migration updates two existing exchange packages and adds three new packages covering new V2 contracts.

## Contract Changes

| Contract | Old Address | New Address | Deploy Block |
|---|---|---|---|
| CTF Exchange | `0x4bFb41d5B3570DeFd03C39a9A4D8dE6Bd8B8982E` | `0xE111180000d2663C0091e4f400237545B87B996B` | 84902353 |
| Neg Risk CTF Exchange | `0xC5d563A36AE78145C45a50134d48A1215220f80a` | `0xe2222d279d744050d28e00520010520000310F59` | 85058176 |
| Conditional Tokens (CTF) | `0x4D97DCd97eC945f40cF65F87097ACe5EA0476045` | **unchanged** | — |
| NegRiskAdapter (new) | — | `0xd91E80cF2E7be2e162c6513ceD06f1dD0dA35296` | 50505403 |
| pUSD token (new) | — | `0xC011a7E12a19f7B1f670d46F03B03f3342E82DFB` | 84902320 |
| CollateralOnramp (new) | — | `0x93070a847efEf7F70739046A929D47a521F5B8ee` | 84902320 |
| CtfCollateralAdapter (new) | — | `0xAdA100Db00Ca00073811820692005400218FcE1f` | 84902320 |
| NegRiskCtfCollateralAdapter (new) | — | `0xadA2005600Dec949baf300f4C6120000bDB6eAab` | 84902320 |
| DepositWalletFactory (new) | — | `0x00000000000Fb5C9ADea0298D729A0CB3823Cc07` | 84902000 |

## V2 Exchange Event Changes (both exchange packages)

### Modified events
- `OrderFilled`: drop `maker_asset_id`/`taker_asset_id`; add `side` (uint8), `token_id` (uint256), `builder` (bytes32), `metadata` (bytes32)
- `OrdersMatched`: drop `maker_asset_id`/`taker_asset_id`; add `side` (uint8), `token_id` (uint256)
- `FeeCharged`: removed `tokenId` parameter; V2 signature is `FeeCharged(address indexed recipient, uint256 amount)`

### Removed events
- `OrderCancelled` (replaced by `UserPaused`/`UserUnpaused`)
- `TokenRegistered`

### New events
- Trading: `OrderPreapproved(orderHash)`, `OrderPreapprovalInvalidated(orderHash)`
- Admin: `NewAdmin`, `NewOperator`, `RemovedAdmin`, `RemovedOperator`
- Fee: `FeeReceiverUpdated(feeReceiver)`, `MaxFeeRateUpdated(maxFeeRate)`
- Pause: `UserPaused(user, effectivePauseBlock)`, `UserUnpaused(user)`, `UserPauseBlockIntervalUpdated(oldInterval, newInterval)`

### polymarket-neg-risk-ctf specific removals
- `ProxyFactoryUpdated`, `SafeFactoryUpdated`

## New Packages

### polymarket-neg-risk-adapter
Contract: `0xd91E80cF2E7be2e162c6513ceD06f1dD0dA35296`  
Events: `MarketPrepared`, `QuestionPrepared`, `OutcomeReported`, `PositionSplit`, `PositionsMerge`, `PositionsConverted`, `PayoutRedemption`, `NewAdmin`, `RemovedAdmin`

### polymarket-collateral
Contracts: pUSD, CollateralOnramp, CtfCollateralAdapter, NegRiskCtfCollateralAdapter  
Events: `Transfer`, `Wrapped`, `Unwrapped`, `PositionSplit`, `PositionsMerged`, `PositionsRedeemed`, `PositionsConverted`

### polymarket-wallet-factory
Contract: `0x00000000000Fb5C9ADea0298D729A0CB3823Cc07`  
Events: `WalletDeployed`, `ImplementationAuthorized`

## Implementation Order

1. Create feature branch (done)
2. Update `polymarket-exchange` 
3. Update `polymarket-neg-risk-ctf`
4. Create `polymarket-neg-risk-adapter`
5. Create `polymarket-collateral`
6. Create `polymarket-wallet-factory`
7. Update Makefile + README
