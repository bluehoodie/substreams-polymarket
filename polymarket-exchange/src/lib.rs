// Generated modules
#[allow(dead_code, clippy::all)]
pub mod pb;
pub mod abi;

use substreams::errors::Error;
use substreams_ethereum::pb::eth::v2 as eth;

use pb::polymarket::exchange::v1 as proto;

/// Ethereum address of the Polymarket CTF Exchange contract on Polygon.
///
/// This is stored as a raw 20-byte array to enable direct byte-level comparison against
/// log addresses, avoiding the overhead of string parsing and heap allocation on every log entry.
///
/// See: <https://polygonscan.com/address/0x4bFb41d5B3570DeFd03C39a9A4D8dE6Bd8B8982E>
const CTF_EXCHANGE_CONTRACT_ADDRESS: [u8; 20] = hex_literal::hex!("4bFb41d5B3570DeFd03C39a9A4D8dE6Bd8B8982E");

/// Map module that extracts CTF Exchange events from blocks
#[substreams::handlers::map]
fn map_exchange_events(blk: eth::Block) -> Result<proto::ExchangeEvents, Error> {
    use abi::ctf_exchange::events::*;

    let mut events = proto::ExchangeEvents::default();

    // Iterate over all logs in the block
    for log in blk.logs() {
        // Filter by contract address first (performance optimization)
        if !is_exchange_contract(log.log) {
            continue;
        }

        // Try to decode each event type (check match_log first to avoid index errors)
        if OrderFilled::match_log(log.log) {
            if let Ok(event) = OrderFilled::decode(log.log) {
                events.order_filled.push(proto::OrderFilled {
                    order_hash: event.order_hash.to_vec(),
                    maker: format_address(&event.maker),
                    taker: format_address(&event.taker),
                    maker_asset_id: bigint_to_string(&event.maker_asset_id),
                    taker_asset_id: bigint_to_string(&event.taker_asset_id),
                    maker_amount_filled: bigint_to_string(&event.maker_amount_filled),
                    taker_amount_filled: bigint_to_string(&event.taker_amount_filled),
                    fee: bigint_to_string(&event.fee),
                    tx: Some(build_transaction_context(&blk, &log)),
                });
            }
        } else if OrderCancelled::match_log(log.log) {
            if let Ok(event) = OrderCancelled::decode(log.log) {
                events.order_cancelled.push(proto::OrderCancelled {
                    order_hash: event.order_hash.to_vec(),
                    tx: Some(build_transaction_context(&blk, &log)),
                });
            }
        } else if OrdersMatched::match_log(log.log) {
            if let Ok(event) = OrdersMatched::decode(log.log) {
                events.orders_matched.push(proto::OrdersMatched {
                    taker_order_hash: event.taker_order_hash.to_vec(),
                    taker_order_maker: format_address(&event.taker_order_maker),
                    maker_asset_id: bigint_to_string(&event.maker_asset_id),
                    taker_asset_id: bigint_to_string(&event.taker_asset_id),
                    maker_amount_filled: bigint_to_string(&event.maker_amount_filled),
                    taker_amount_filled: bigint_to_string(&event.taker_amount_filled),
                    tx: Some(build_transaction_context(&blk, &log)),
                });
            }
        }
        // If none of the decoders match, silently skip (resilient error handling)
    }

    Ok(events)
}

/// Map module that extracts Registry events from blocks
#[substreams::handlers::map]
fn map_registry_events(blk: eth::Block) -> Result<proto::RegistryEvents, Error> {
    use abi::ctf_exchange::events::*;

    let mut events = proto::RegistryEvents::default();

    // Iterate over all logs in the block
    for log in blk.logs() {
        // Filter by contract address first (performance optimization)
        if !is_exchange_contract(log.log) {
            continue;
        }

        // Try to decode TokenRegistered event (check match_log first to avoid index errors)
        if TokenRegistered::match_log(log.log) {
            if let Ok(event) = TokenRegistered::decode(log.log) {
                events.token_registered.push(proto::TokenRegistered {
                    token0: bigint_to_string(&event.token0),
                    token1: bigint_to_string(&event.token1),
                    condition_id: event.condition_id.to_vec(),
                    tx: Some(build_transaction_context(&blk, &log)),
                });
            }
        }
        // If decoder doesn't match, silently skip (resilient error handling)
    }

    Ok(events)
}

/// Map module that extracts FeeCharged events from blocks
#[substreams::handlers::map]
fn map_fee_events(blk: eth::Block) -> Result<proto::FeeEvents, Error> {
    use abi::ctf_exchange::events::*;

    let mut events = proto::FeeEvents::default();

    // Iterate over all logs in the block
    for log in blk.logs() {
        // Filter by contract address first (performance optimization)
        if !is_exchange_contract(log.log) {
            continue;
        }

        // Try to decode FeeCharged event (check match_log first to avoid index errors)
        if FeeCharged::match_log(log.log) {
            if let Ok(event) = FeeCharged::decode(log.log) {
                events.fee_charged.push(proto::FeeCharged {
                    recipient: format_address(&event.recipient),
                    token_id: bigint_to_string(&event.token_id),
                    amount: bigint_to_string(&event.amount),
                    tx: Some(build_transaction_context(&blk, &log)),
                });
            }
        }
        // If decoder doesn't match, silently skip (resilient error handling)
    }

    Ok(events)
}

/// Unified map module that extracts all event types in a single pass through block logs.
///
/// This module processes all Exchange, Registry, and Fee events in one iteration,
/// providing better performance than composing separate modules.
///
/// # Performance
/// By processing all event types in a single pass, this module eliminates redundant
/// iterations over the same log data, improving processing efficiency.
///
/// # Arguments
/// * `blk` - The Ethereum block to extract events from
///
/// # Returns
/// AllEvents message containing all extracted event types with optional fields
#[substreams::handlers::map]
pub fn map_all_events(blk: eth::Block) -> Result<proto::AllEvents, Error> {
    use abi::ctf_exchange::events::*;

    // Initialize all event collectors
    let mut exchange_events = proto::ExchangeEvents::default();
    let mut registry_events = proto::RegistryEvents::default();
    let mut fee_events = proto::FeeEvents::default();

    // Single pass through all logs, extracting all event types
    for log in blk.logs() {
        // Filter by contract address first (performance optimization)
        if !is_exchange_contract(log.log) {
            continue;
        }

        // Exchange events
        if OrderFilled::match_log(log.log) {
            if let Ok(event) = OrderFilled::decode(log.log) {
                exchange_events.order_filled.push(proto::OrderFilled {
                    order_hash: event.order_hash.to_vec(),
                    maker: format_address(&event.maker),
                    taker: format_address(&event.taker),
                    maker_asset_id: bigint_to_string(&event.maker_asset_id),
                    taker_asset_id: bigint_to_string(&event.taker_asset_id),
                    maker_amount_filled: bigint_to_string(&event.maker_amount_filled),
                    taker_amount_filled: bigint_to_string(&event.taker_amount_filled),
                    fee: bigint_to_string(&event.fee),
                    tx: Some(build_transaction_context(&blk, &log)),
                });
            }
        } else if OrderCancelled::match_log(log.log) {
            if let Ok(event) = OrderCancelled::decode(log.log) {
                exchange_events.order_cancelled.push(proto::OrderCancelled {
                    order_hash: event.order_hash.to_vec(),
                    tx: Some(build_transaction_context(&blk, &log)),
                });
            }
        } else if OrdersMatched::match_log(log.log) {
            if let Ok(event) = OrdersMatched::decode(log.log) {
                exchange_events.orders_matched.push(proto::OrdersMatched {
                    taker_order_hash: event.taker_order_hash.to_vec(),
                    taker_order_maker: format_address(&event.taker_order_maker),
                    maker_asset_id: bigint_to_string(&event.maker_asset_id),
                    taker_asset_id: bigint_to_string(&event.taker_asset_id),
                    maker_amount_filled: bigint_to_string(&event.maker_amount_filled),
                    taker_amount_filled: bigint_to_string(&event.taker_amount_filled),
                    tx: Some(build_transaction_context(&blk, &log)),
                });
            }
        }
        // Registry events
        else if TokenRegistered::match_log(log.log) {
            if let Ok(event) = TokenRegistered::decode(log.log) {
                registry_events.token_registered.push(proto::TokenRegistered {
                    token0: bigint_to_string(&event.token0),
                    token1: bigint_to_string(&event.token1),
                    condition_id: event.condition_id.to_vec(),
                    tx: Some(build_transaction_context(&blk, &log)),
                });
            }
        }
        // Fee events
        else if FeeCharged::match_log(log.log) {
            if let Ok(event) = FeeCharged::decode(log.log) {
                fee_events.fee_charged.push(proto::FeeCharged {
                    recipient: format_address(&event.recipient),
                    token_id: bigint_to_string(&event.token_id),
                    amount: bigint_to_string(&event.amount),
                    tx: Some(build_transaction_context(&blk, &log)),
                });
            }
        }
        // If none of the decoders match, silently skip (resilient error handling)
    }

    // Build and return AllEvents with optional fields for non-empty collections
    Ok(proto::AllEvents {
        exchange_events: if !exchange_events.order_filled.is_empty()
            || !exchange_events.order_cancelled.is_empty()
            || !exchange_events.orders_matched.is_empty()
        {
            Some(exchange_events)
        } else {
            None
        },
        registry_events: if !registry_events.token_registered.is_empty() {
            Some(registry_events)
        } else {
            None
        },
        fee_events: if !fee_events.fee_charged.is_empty() {
            Some(fee_events)
        } else {
            None
        },
    })
}

// Helper functions

/// Safely converts a [`BigInt`](substreams::scalar::BigInt) to its decimal string representation.
///
/// Handles an edge case where `BigInt::to_string()` may return an empty string
/// for certain internal states (e.g., zero-length byte representations). In such cases,
/// this function returns `"0"` as a safe default.
///
/// This wrapper exists to prevent downstream protobuf serialization issues where
/// an empty string in a numeric field would be invalid.
#[inline]
fn bigint_to_string(bigint: &substreams::scalar::BigInt) -> String {
    let s = bigint.to_string();
    if s.is_empty() {
        "0".to_string()
    } else {
        s
    }
}

/// Checks if a log originates from the CTF Exchange contract using byte-level comparison.
///
/// Compares the log's 20-byte address directly against [`CTF_EXCHANGE_CONTRACT_ADDRESS`].
/// This is significantly faster than string-based comparison (~25x) because it avoids
/// hex encoding, string allocation, and case-insensitive matching entirely.
#[inline]
fn is_exchange_contract(log: &eth::Log) -> bool {
    log.address == CTF_EXCHANGE_CONTRACT_ADDRESS
}

/// Formats raw bytes as a lowercase hex address string with `0x` prefix.
///
/// Converts an arbitrary byte slice (typically a 20-byte Ethereum address) into its
/// hex-encoded string representation. The output is always lowercase.
///
/// # Examples
///
/// ```ignore
/// let addr = hex_literal::hex!("4bFb41d5B3570DeFd03C39a9A4D8dE6Bd8B8982E");
/// assert_eq!(format_address(&addr), "0x4bfb41d5b3570defd03c39a9a4d8de6bd8b8982e");
/// ```
#[inline]
fn format_address(bytes: &[u8]) -> String {
    format!("0x{}", hex::encode(bytes))
}

/// Builds a [`proto::TransactionContext`] from block and log data.
///
/// Extracts transaction metadata needed to identify and locate an on-chain event:
/// - `tx_hash`: The transaction hash, hex-encoded with `0x` prefix.
/// - `log_index`: The log's position within the block (derived from `block_index`).
/// - `block_number`: The block height.
/// - `timestamp`: The block's Unix timestamp in seconds.
///
/// This context is attached to every emitted event to enable downstream consumers
/// to trace events back to their originating transactions.
#[inline]
fn build_transaction_context(blk: &eth::Block, log: &substreams_ethereum::block_view::LogView) -> proto::TransactionContext {
    proto::TransactionContext {
        tx_hash: format_address(&log.receipt.transaction.hash),
        log_index: log.log.block_index as u64,
        block_number: blk.number,
        timestamp: blk.timestamp_seconds(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use substreams_ethereum::pb::eth::v2 as eth;

    #[test]
    fn test_bigint_zero_to_string() {
        let zero = substreams::scalar::BigInt::from(0u64);
        assert_eq!(zero.to_string(), "0");
    }

    #[test]
    fn test_bigint_from_bytes_zero() {
        let zeros = [0u8; 32];
        let bigint = substreams::scalar::BigInt::from_unsigned_bytes_be(&zeros);
        assert_eq!(bigint.to_string(), "0");
    }

    #[test]
    fn test_is_exchange_contract_matching_address() {
        let log = eth::Log {
            address: hex_literal::hex!("4bFb41d5B3570DeFd03C39a9A4D8dE6Bd8B8982E").to_vec(),
            ..Default::default()
        };
        assert!(is_exchange_contract(&log));
    }

    #[test]
    fn test_is_exchange_contract_non_matching_address() {
        let log = eth::Log {
            address: hex_literal::hex!("0000000000000000000000000000000000000000").to_vec(),
            ..Default::default()
        };
        assert!(!is_exchange_contract(&log));
    }

    #[test]
    fn test_exchange_contract_address_is_20_bytes() {
        assert_eq!(CTF_EXCHANGE_CONTRACT_ADDRESS.len(), 20);
    }

    #[test]
    fn test_format_address() {
        let bytes = hex_literal::hex!("4bFb41d5B3570DeFd03C39a9A4D8dE6Bd8B8982E");
        let result = format_address(&bytes);
        assert_eq!(result, "0x4bfb41d5b3570defd03c39a9a4d8de6bd8b8982e");
    }

    #[test]
    fn test_format_address_zero() {
        let bytes = [0u8; 20];
        let result = format_address(&bytes);
        assert_eq!(result, "0x0000000000000000000000000000000000000000");
    }

    #[test]
    fn test_format_address_empty() {
        let bytes: [u8; 0] = [];
        let result = format_address(&bytes);
        assert_eq!(result, "0x");
    }
}
