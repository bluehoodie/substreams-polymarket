// Generated ABI bindings (via build.rs / substreams-ethereum Abigen). Not every
// decoded event variant is referenced by the user-address extraction below, so
// `dead_code` is allowed; the per-file `#![allow(clippy::all)]` covers clippy.
#[allow(dead_code, clippy::all)]
pub mod abi;

use std::collections::HashSet;

use substreams::errors::Error;
use substreams::scalar::BigInt;
use substreams_ethereum::pb::eth::v2 as eth;
use substreams_ethereum::pb::eth::v2::__buffa::lazy_view::LogLazyView;
use substreams_ethereum::LogLike;

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
pub fn bigint_to_u64(bigint: &BigInt) -> u64 {
    // `BigInt::to_u64()` panics on out-of-range values, so parse the decimal
    // string instead: any value that doesn't fit a u64 (or is negative) yields
    // 0 via `unwrap_or`, which cannot panic in a handler.
    bigint.to_string().parse::<u64>().unwrap_or(0)
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
pub fn tx_context(
    tx_hash: &[u8],
    log: &LogLazyView,
    block_number: u64,
    timestamp: u64,
) -> TxContext {
    TxContext {
        tx_hash: format_address(tx_hash),
        log_index: log.block_index as u64,
        block_number,
        timestamp,
    }
}

/// The block header's timestamp in seconds, read from a lazy block view.
///
/// Errors (rather than panicking like the owned `Block::timestamp_seconds`) when
/// the header fails to decode or is absent.
pub fn block_timestamp_seconds(blk: &eth::BlockLazyView) -> Result<u64, Error> {
    let header = blk
        .header
        .get()?
        .ok_or_else(|| Error::msg(format!("block {} has no header", blk.number)))?;
    let ts = header
        .timestamp
        .as_option()
        .ok_or_else(|| Error::msg(format!("block {} header has no timestamp", blk.number)))?;
    Ok(ts.seconds as u64)
}

/// Runs `f` over a lazy view of an owned block, for tests and benches whose
/// fixtures are owned `eth::Block`s: encodes the block and decodes it lazily,
/// as the runtime does for a `&eth::BlockLazyView<'_>` handler.
pub fn with_lazy_block<R>(
    blk: &eth::Block,
    f: impl FnOnce(&eth::BlockLazyView<'_>) -> R,
) -> Result<R, buffa::DecodeError> {
    use buffa::view::LazyMessageView;
    use buffa::Message;
    let bytes = blk.encode_to_vec();
    let view = eth::BlockLazyView::decode_lazy(&bytes)?;
    Ok(f(&view))
}

/// Loads a real-block fixture captured with
/// `firecore tools firehose-single-block-client <endpoint> <block> -o bytes --bytes-encoding=base64`
/// (see `scripts/capture_fixture.sh`) and decodes it into an owned `eth::Block`.
/// Test-only: behind the `fixtures` feature, which packages enable from
/// `[dev-dependencies]`.
#[cfg(feature = "fixtures")]
pub fn load_block_b64(
    path: impl AsRef<std::path::Path>,
) -> Result<eth::Block, Box<dyn std::error::Error>> {
    use base64::{engine::general_purpose::STANDARD, Engine as _};
    use buffa::Message;
    let raw = std::fs::read_to_string(path.as_ref())?;
    let bytes = STANDARD.decode(raw.trim())?;
    Ok(eth::Block::decode_from_slice(&bytes)?)
}

/// A log of a successful transaction, paired with that transaction's hash.
pub struct BlockLog<'a> {
    pub tx_hash: &'a [u8],
    pub log: LogLazyView<'a>,
}

/// The logs a handler kept from one block, with the block fields its
/// transaction contexts need.
pub struct BlockLogs<'a> {
    pub number: u64,
    /// Header timestamp in seconds; read only when at least one log was kept
    /// (0 otherwise), so a block with nothing to emit never touches the header.
    pub timestamp: u64,
    pub logs: Vec<BlockLog<'a>>,
}

/// Walks the logs in the receipts of the block's successful transactions, in
/// block order, and keeps those whose emitting contract address satisfies
/// `keep`, each paired with its transaction hash.
///
/// This mirrors the owned `Block::logs()` (whose `LogView` carries the
/// transaction) but returns decode errors instead of panicking, and rejects on
/// `address` before anything is decoded or stored. A successful transaction
/// without a receipt contributes no logs.
pub fn block_logs<'a>(
    blk: &eth::BlockLazyView<'a>,
    keep: impl Fn(&[u8]) -> bool,
) -> Result<BlockLogs<'a>, Error> {
    let mut logs = Vec::new();
    for tx in blk.transaction_traces.iter() {
        let tx = tx?;
        if tx.status != eth::TransactionTraceStatus::Succeeded {
            continue;
        }
        let Some(receipt) = tx.receipt.get()? else {
            continue;
        };
        for log in receipt.logs.iter() {
            let log = log?;
            if keep(log.address) {
                logs.push(BlockLog {
                    tx_hash: tx.hash,
                    log,
                });
            }
        }
    }
    let timestamp = if logs.is_empty() {
        0
    } else {
        block_timestamp_seconds(blk)?
    };
    Ok(BlockLogs {
        number: blk.number,
        timestamp,
        logs,
    })
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
pub fn user_addresses_for_log<L: LogLike>(address: &[u8], log: &L) -> Vec<Vec<u8>> {
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
pub fn collect_user_addresses(blk: &eth::BlockLazyView) -> Result<Vec<Vec<u8>>, Error> {
    let mut set: HashSet<Vec<u8>> = HashSet::new();
    let kept = block_logs(blk, |address| {
        USER_ACTIVITY_CONTRACTS.iter().any(|c| address == c)
    })?;
    for BlockLog { log, .. } in &kept.logs {
        for addr in user_addresses_for_log(log.address, log) {
            set.insert(addr);
        }
    }
    let mut addrs: Vec<Vec<u8>> = set.into_iter().collect();
    addrs.sort();
    Ok(addrs)
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
        let got = with_lazy_block(&blk, collect_user_addresses)
            .expect("valid block")
            .expect("no decode error");
        assert!(got.is_empty());
    }
}
