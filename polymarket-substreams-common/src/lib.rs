// Generated ABI bindings (via build.rs / substreams-ethereum Abigen). Not every
// decoded event variant is referenced by the user-address extraction below, so
// `dead_code` is allowed; the per-file `#![allow(clippy::all)]` covers clippy.
#[allow(dead_code, clippy::all)]
pub mod abi;

use std::collections::HashSet;

use substreams::scalar::BigInt;
use substreams_ethereum::block_view::LogView;
use substreams_ethereum::pb::eth::v2 as eth;

#[inline]
pub fn bigint_to_string(bigint: &BigInt) -> String {
    let s = bigint.to_string();
    if s.is_empty() {
        "0".to_string()
    } else {
        s
    }
}

#[inline]
pub fn bigint_to_u32(bigint: &BigInt) -> u32 {
    // `BigInt::to_u64()` panics on out-of-range values, so parse the decimal
    // string instead: any value that doesn't fit a u32 (or is negative) yields
    // 0 via `unwrap_or`, which cannot panic in a handler.
    bigint.to_string().parse::<u32>().unwrap_or(0)
}

#[inline]
pub fn format_address(bytes: &[u8]) -> String {
    format!("0x{}", hex::encode(bytes))
}

/// Primitive transaction-context fields, package-agnostic.
/// Each substreams package wraps this into its own `proto::TransactionContext`.
pub struct TxContext {
    pub tx_hash: String,
    pub log_index: u64,
    pub block_number: u64,
    pub timestamp: u64,
}

#[inline]
pub fn build_tx_context(blk: &eth::Block, log: &LogView) -> TxContext {
    TxContext {
        tx_hash: format_address(&log.receipt.transaction.hash),
        log_index: log.log.block_index as u64,
        block_number: blk.number,
        timestamp: blk.timestamp_seconds(),
    }
}

// ---------------------------------------------------------------------------
// Polymarket user-activity address extraction.
//
// Single source of truth for "which wallets took an action in this block".
// Consumers (the trader index) build `user:<addr>` block-index keys from the
// raw 20-byte addresses returned here. Returning bytes (not formatted keys)
// keeps this layer presentation-agnostic: the `user:` namespace is the
// consumer's concern.
//
// Resolution (UMA oracle) contracts are intentionally excluded — disputes /
// proposals are protocol-level, not a wallet's trading footprint.
// ---------------------------------------------------------------------------

/// CTF Exchange (CLOB).
pub const CTF_EXCHANGE: [u8; 20] = hex_literal::hex!("E111180000d2663C0091e4f400237545B87B996B");
/// Neg-Risk CTF Exchange.
pub const NEG_RISK_CTF: [u8; 20] = hex_literal::hex!("e2222d279d744050d28e00520010520000310F59");
/// Gnosis Conditional Tokens Framework (ERC1155).
pub const CONDITIONAL_TOKENS: [u8; 20] =
    hex_literal::hex!("4D97DCd97eC945f40cF65F87097ACe5EA0476045");
/// Neg-Risk Adapter.
pub const NEG_RISK_ADAPTER: [u8; 20] =
    hex_literal::hex!("d91E80cF2E7be2e162c6513ceD06f1dD0dA35296");
/// Polymarket USD (pUSD) collateral token.
pub const PUSD: [u8; 20] = hex_literal::hex!("C011a7E12a19f7B1f670d46F03B03f3342E82DFB");
/// CTF collateral adapter.
pub const CTF_COLLATERAL_ADAPTER: [u8; 20] =
    hex_literal::hex!("AdA100Db00Ca00073811820692005400218FcE1f");
/// Neg-Risk CTF collateral adapter.
pub const NEG_RISK_CTF_COLLATERAL_ADAPTER: [u8; 20] =
    hex_literal::hex!("adA2005600Dec949baf300f4C6120000bDB6eAab");
/// Deposit wallet (proxy) factory.
pub const DEPOSIT_WALLET_FACTORY: [u8; 20] =
    hex_literal::hex!("00000000000Fb5C9ADea0298D729A0CB3823Cc07");

/// All Polymarket contracts that carry user-attributable activity. Useful for
/// callers that build a contract-address block-filter query (`evt_addr:…`).
pub const USER_ACTIVITY_CONTRACTS: [[u8; 20]; 8] = [
    CTF_EXCHANGE,
    NEG_RISK_CTF,
    CONDITIONAL_TOKENS,
    NEG_RISK_ADAPTER,
    PUSD,
    CTF_COLLATERAL_ADAPTER,
    NEG_RISK_CTF_COLLATERAL_ADAPTER,
    DEPOSIT_WALLET_FACTORY,
];

/// Extracts every user-attributable address from a single raw log, dispatching
/// on the emitting contract `address`. Returns raw 20-byte addresses (possibly
/// with duplicates within one log, e.g. self-transfers). Returns an empty vec
/// for logs from non-Polymarket contracts or events we do not attribute.
pub fn user_addresses_for_log(address: &[u8], log: &eth::Log) -> Vec<Vec<u8>> {
    let mut out: Vec<Vec<u8>> = Vec::new();

    if address == CTF_EXCHANGE {
        use abi::ctf_exchange::events as ex;
        if ex::OrderFilled::match_log(log) {
            if let Ok(e) = ex::OrderFilled::decode(log) {
                out.push(e.maker);
                out.push(e.taker);
            }
        } else if ex::OrdersMatched::match_log(log) {
            if let Ok(e) = ex::OrdersMatched::decode(log) {
                out.push(e.taker_order_maker);
            }
        }
    } else if address == NEG_RISK_CTF {
        use abi::neg_risk_ctf::events as nr;
        if nr::OrderFilled::match_log(log) {
            if let Ok(e) = nr::OrderFilled::decode(log) {
                out.push(e.maker);
                out.push(e.taker);
            }
        } else if nr::OrdersMatched::match_log(log) {
            if let Ok(e) = nr::OrdersMatched::decode(log) {
                out.push(e.taker_order_maker);
            }
        }
    } else if address == CONDITIONAL_TOKENS {
        use abi::conditional_tokens::events as ct;
        if ct::PositionSplit::match_log(log) {
            if let Ok(e) = ct::PositionSplit::decode(log) {
                out.push(e.stakeholder);
            }
        } else if ct::PositionsMerge::match_log(log) {
            if let Ok(e) = ct::PositionsMerge::decode(log) {
                out.push(e.stakeholder);
            }
        } else if ct::PayoutRedemption::match_log(log) {
            if let Ok(e) = ct::PayoutRedemption::decode(log) {
                out.push(e.redeemer);
            }
        } else if ct::TransferSingle::match_log(log) {
            if let Ok(e) = ct::TransferSingle::decode(log) {
                out.push(e.from);
                out.push(e.to);
                out.push(e.operator);
            }
        } else if ct::TransferBatch::match_log(log) {
            if let Ok(e) = ct::TransferBatch::decode(log) {
                out.push(e.from);
                out.push(e.to);
                out.push(e.operator);
            }
        } else if ct::ApprovalForAll::match_log(log) {
            if let Ok(e) = ct::ApprovalForAll::decode(log) {
                out.push(e.owner);
            }
        }
    } else if address == NEG_RISK_ADAPTER {
        use abi::neg_risk_adapter::events as na;
        if na::PositionSplit::match_log(log) {
            if let Ok(e) = na::PositionSplit::decode(log) {
                out.push(e.stakeholder);
            }
        } else if na::PositionsMerge::match_log(log) {
            if let Ok(e) = na::PositionsMerge::decode(log) {
                out.push(e.stakeholder);
            }
        } else if na::PositionsConverted::match_log(log) {
            if let Ok(e) = na::PositionsConverted::decode(log) {
                out.push(e.stakeholder);
            }
        } else if na::PayoutRedemption::match_log(log) {
            if let Ok(e) = na::PayoutRedemption::decode(log) {
                out.push(e.redeemer);
            }
        }
    } else if address == PUSD {
        use abi::p_usd::events as pu;
        if pu::Transfer::match_log(log) {
            if let Ok(e) = pu::Transfer::decode(log) {
                out.push(e.from);
                out.push(e.to);
            }
        } else if pu::Wrapped::match_log(log) {
            if let Ok(e) = pu::Wrapped::decode(log) {
                out.push(e.caller);
                out.push(e.to);
            }
        } else if pu::Unwrapped::match_log(log) {
            if let Ok(e) = pu::Unwrapped::decode(log) {
                out.push(e.caller);
                out.push(e.to);
            }
        }
    } else if address == CTF_COLLATERAL_ADAPTER {
        use abi::ctf_collateral_adapter::events as ca;
        if ca::PositionSplit::match_log(log) {
            if let Ok(e) = ca::PositionSplit::decode(log) {
                out.push(e.initiator);
            }
        } else if ca::PositionsMerged::match_log(log) {
            if let Ok(e) = ca::PositionsMerged::decode(log) {
                out.push(e.initiator);
            }
        } else if ca::PositionsRedeemed::match_log(log) {
            if let Ok(e) = ca::PositionsRedeemed::decode(log) {
                out.push(e.initiator);
            }
        }
    } else if address == NEG_RISK_CTF_COLLATERAL_ADAPTER {
        use abi::neg_risk_ctf_collateral_adapter::events as nca;
        if nca::PositionSplit::match_log(log) {
            if let Ok(e) = nca::PositionSplit::decode(log) {
                out.push(e.initiator);
            }
        } else if nca::PositionsMerged::match_log(log) {
            if let Ok(e) = nca::PositionsMerged::decode(log) {
                out.push(e.initiator);
            }
        } else if nca::PositionsRedeemed::match_log(log) {
            if let Ok(e) = nca::PositionsRedeemed::decode(log) {
                out.push(e.initiator);
            }
        } else if nca::PositionsConverted::match_log(log) {
            if let Ok(e) = nca::PositionsConverted::decode(log) {
                out.push(e.initiator);
            }
        }
    } else if address == DEPOSIT_WALLET_FACTORY {
        use abi::deposit_wallet_factory::events as wf;
        if wf::WalletDeployed::match_log(log) {
            if let Ok(e) = wf::WalletDeployed::decode(log) {
                out.push(e.wallet);
                out.push(e.owner);
            }
        }
    }

    out
}

/// Aggregates every user-attributable address across all Polymarket contracts
/// for a block. Single pass over `blk.logs()`. Returns deduplicated raw 20-byte
/// addresses in deterministic (sorted) order — `HashSet` iteration order is
/// unspecified, so sorting keeps the derived block index reproducible.
pub fn collect_user_addresses(blk: &eth::Block) -> Vec<Vec<u8>> {
    let mut set: HashSet<Vec<u8>> = HashSet::new();
    for log in blk.logs() {
        for addr in user_addresses_for_log(&log.log.address, log.log) {
            set.insert(addr);
        }
    }
    let mut addrs: Vec<Vec<u8>> = set.into_iter().collect();
    addrs.sort();
    addrs
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bigint_to_string_zero() {
        let zero = BigInt::from(0u64);
        // BigInt::from(0) gives "0", not empty — still correct output
        assert_eq!(bigint_to_string(&zero), "0");
    }

    #[test]
    fn test_bigint_to_string_normal() {
        let val = BigInt::from(12345u64);
        assert_eq!(bigint_to_string(&val), "12345");
    }

    #[test]
    fn test_bigint_to_string_large() {
        let val = BigInt::from(u64::MAX);
        assert_eq!(bigint_to_string(&val), u64::MAX.to_string());
    }

    #[test]
    fn test_format_address_known() {
        let bytes: [u8; 4] = [0xde, 0xad, 0xbe, 0xef];
        assert_eq!(format_address(&bytes), "0xdeadbeef");
    }

    #[test]
    fn test_format_address_zero() {
        let bytes: [u8; 4] = [0x00, 0x00, 0x00, 0x00];
        assert_eq!(format_address(&bytes), "0x00000000");
    }

    #[test]
    fn test_bigint_to_u32_zero() {
        let val = BigInt::from(0u64);
        assert_eq!(bigint_to_u32(&val), 0u32);
    }

    #[test]
    fn test_bigint_to_u32_one() {
        let val = BigInt::from(1u64);
        assert_eq!(bigint_to_u32(&val), 1u32);
    }

    #[test]
    fn test_bigint_to_u32_max() {
        let val = BigInt::from(u32::MAX as u64);
        assert_eq!(bigint_to_u32(&val), u32::MAX);
    }

    #[test]
    fn test_bigint_to_u32_overflow_saturates_to_zero() {
        // A uint256 beyond u32::MAX must not panic — it maps to 0 rather than
        // truncating or aborting the handler.
        let val = BigInt::from(u32::MAX as u64 + 1);
        assert_eq!(bigint_to_u32(&val), 0);
    }
}

#[cfg(test)]
mod user_activity_tests {
    use super::*;

    // Canonical keccak256("TransferSingle(address,address,address,uint256,uint256)").
    // ERC-1155 TransferSingle topic0 — a stable, well-known signature hash. Used to
    // build a genuinely decodable log so the full match_log -> decode -> extract path
    // is exercised end to end (not just the address dispatch).
    const TRANSFER_SINGLE_TOPIC0: [u8; 32] =
        hex_literal::hex!("c3d58168c5ae7397731d063d5bbf3d657854427343f4c083240f7aacaa2d0f62");

    /// Left-pads a 20-byte address into a 32-byte EVM topic word.
    fn topic_addr(addr: &[u8; 20]) -> Vec<u8> {
        let mut t = vec![0u8; 12];
        t.extend_from_slice(addr);
        t
    }

    /// Builds a valid ConditionalTokens `TransferSingle` log: 4 topics
    /// (topic0, operator, from, to) + 64 bytes of data (id, value).
    fn transfer_single_log(operator: &[u8; 20], from: &[u8; 20], to: &[u8; 20]) -> eth::Log {
        eth::Log {
            address: CONDITIONAL_TOKENS.to_vec(),
            topics: vec![
                TRANSFER_SINGLE_TOPIC0.to_vec(),
                topic_addr(operator),
                topic_addr(from),
                topic_addr(to),
            ],
            data: vec![0u8; 64], // id (uint256) || value (uint256)
            index: 0,
            block_index: 0,
            ordinal: 0,
        }
    }

    #[test]
    fn test_all_contracts_distinct_and_20_bytes() {
        for (i, a) in USER_ACTIVITY_CONTRACTS.iter().enumerate() {
            assert_eq!(a.len(), 20);
            for b in USER_ACTIVITY_CONTRACTS.iter().skip(i + 1) {
                assert_ne!(a, b);
            }
        }
    }

    #[test]
    fn test_unknown_contract_address_yields_nothing() {
        let log = transfer_single_log(&[1u8; 20], &[2u8; 20], &[3u8; 20]);
        // Dispatch on an address we do not track -> empty, even though the log
        // itself is a valid TransferSingle.
        let unknown = [0xaau8; 20];
        assert!(user_addresses_for_log(&unknown, &log).is_empty());
    }

    #[test]
    fn test_transfer_single_extracts_from_to_operator() {
        let operator = [0x11u8; 20];
        let from = [0x22u8; 20];
        let to = [0x33u8; 20];
        let log = transfer_single_log(&operator, &from, &to);

        let got = user_addresses_for_log(&CONDITIONAL_TOKENS, &log);
        // Order within a log follows push order: from, to, operator.
        assert_eq!(got, vec![from.to_vec(), to.to_vec(), operator.to_vec()]);
    }

    #[test]
    fn test_collect_user_addresses_empty_block() {
        let blk = eth::Block::default();
        assert!(collect_user_addresses(&blk).is_empty());
    }
}
