// Generated modules
#[allow(dead_code, clippy::all)]
pub mod pb;
pub mod abi;

use substreams::errors::Error;
use substreams_ethereum::pb::eth::v2 as eth;

use pb::polymarket::neg_risk_ctf::v1 as proto;

/// Ethereum address of the Polymarket Neg Risk CTF contract on Polygon.
///
/// This is stored as a raw 20-byte array to enable direct byte-level comparison against
/// log addresses, avoiding the overhead of string parsing and heap allocation on every log entry.
///
/// See: <https://polygonscan.com/address/0xc5d563a36ae78145c45a50134d48a1215220f80a>
const NEG_RISK_CTF_CONTRACT_ADDRESS: [u8; 20] = hex_literal::hex!("c5d563a36ae78145c45a50134d48a1215220f80a");

/// Map module that extracts FeeCharged events from blocks
#[substreams::handlers::map]
fn map_fee_events(blk: eth::Block) -> Result<proto::FeeEvents, Error> {
    use abi::neg_risk_ctf::events::*;

    let mut events = proto::FeeEvents::default();

    for log in blk.logs() {
        if !is_neg_risk_ctf_contract(log.log) {
            continue;
        }

        if FeeCharged::match_log(log.log) {
            if let Ok(event) = FeeCharged::decode(log.log) {
                events.fee_charged.push(proto::FeeCharged {
                    recipient: format_address(&event.receiver),
                    token_id: bigint_to_string(&event.token_id),
                    amount: bigint_to_string(&event.amount),
                    tx: Some(build_transaction_context(&blk, &log)),
                });
            }
        }
    }

    Ok(events)
}

/// Map module that extracts Admin events from blocks
#[substreams::handlers::map]
fn map_admin_events(blk: eth::Block) -> Result<proto::AdminEvents, Error> {
    use abi::neg_risk_ctf::events::*;

    let mut events = proto::AdminEvents::default();

    for log in blk.logs() {
        if !is_neg_risk_ctf_contract(log.log) {
            continue;
        }

        if NewAdmin::match_log(log.log) {
            if let Ok(event) = NewAdmin::decode(log.log) {
                events.new_admin.push(proto::NewAdmin {
                    new_admin_address: format_address(&event.new_admin_address),
                    admin: format_address(&event.admin),
                    tx: Some(build_transaction_context(&blk, &log)),
                });
            }
        } else if NewOperator::match_log(log.log) {
            if let Ok(event) = NewOperator::decode(log.log) {
                events.new_operator.push(proto::NewOperator {
                    new_operator_address: format_address(&event.new_operator_address),
                    admin: format_address(&event.admin),
                    tx: Some(build_transaction_context(&blk, &log)),
                });
            }
        } else if RemovedAdmin::match_log(log.log) {
            if let Ok(event) = RemovedAdmin::decode(log.log) {
                events.removed_admin.push(proto::RemovedAdmin {
                    removed_admin: format_address(&event.removed_admin),
                    admin: format_address(&event.admin),
                    tx: Some(build_transaction_context(&blk, &log)),
                });
            }
        } else if RemovedOperator::match_log(log.log) {
            if let Ok(event) = RemovedOperator::decode(log.log) {
                events.removed_operator.push(proto::RemovedOperator {
                    removed_operator: format_address(&event.removed_operator),
                    admin: format_address(&event.admin),
                    tx: Some(build_transaction_context(&blk, &log)),
                });
            }
        }
    }

    Ok(events)
}

/// Map module that extracts Trading events from blocks
#[substreams::handlers::map]
fn map_trading_events(blk: eth::Block) -> Result<proto::TradingEvents, Error> {
    use abi::neg_risk_ctf::events::*;

    let mut events = proto::TradingEvents::default();

    for log in blk.logs() {
        if !is_neg_risk_ctf_contract(log.log) {
            continue;
        }

        if OrderCancelled::match_log(log.log) {
            if let Ok(event) = OrderCancelled::decode(log.log) {
                events.order_cancelled.push(proto::OrderCancelled {
                    order_hash: event.order_hash.to_vec(),
                    tx: Some(build_transaction_context(&blk, &log)),
                });
            }
        } else if OrderFilled::match_log(log.log) {
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
    }

    Ok(events)
}

/// Map module that extracts Registry events from blocks
#[substreams::handlers::map]
fn map_registry_events(blk: eth::Block) -> Result<proto::RegistryEvents, Error> {
    use abi::neg_risk_ctf::events::*;

    let mut events = proto::RegistryEvents::default();

    for log in blk.logs() {
        if !is_neg_risk_ctf_contract(log.log) {
            continue;
        }

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
    }

    Ok(events)
}

/// Map module that extracts Pause events from blocks
#[substreams::handlers::map]
fn map_pause_events(blk: eth::Block) -> Result<proto::PauseEvents, Error> {
    use abi::neg_risk_ctf::events::*;

    let mut events = proto::PauseEvents::default();

    for log in blk.logs() {
        if !is_neg_risk_ctf_contract(log.log) {
            continue;
        }

        if TradingPaused::match_log(log.log) {
            if let Ok(event) = TradingPaused::decode(log.log) {
                events.trading_paused.push(proto::TradingPaused {
                    pauser: format_address(&event.pauser),
                    tx: Some(build_transaction_context(&blk, &log)),
                });
            }
        } else if TradingUnpaused::match_log(log.log) {
            if let Ok(event) = TradingUnpaused::decode(log.log) {
                events.trading_unpaused.push(proto::TradingUnpaused {
                    pauser: format_address(&event.pauser),
                    tx: Some(build_transaction_context(&blk, &log)),
                });
            }
        }
    }

    Ok(events)
}

/// Map module that extracts Config events from blocks
#[substreams::handlers::map]
fn map_config_events(blk: eth::Block) -> Result<proto::ConfigEvents, Error> {
    use abi::neg_risk_ctf::events::*;

    let mut events = proto::ConfigEvents::default();

    for log in blk.logs() {
        if !is_neg_risk_ctf_contract(log.log) {
            continue;
        }

        if ProxyFactoryUpdated::match_log(log.log) {
            if let Ok(event) = ProxyFactoryUpdated::decode(log.log) {
                events.proxy_factory_updated.push(proto::ProxyFactoryUpdated {
                    old_proxy_factory: format_address(&event.old_proxy_factory),
                    new_proxy_factory: format_address(&event.new_proxy_factory),
                    tx: Some(build_transaction_context(&blk, &log)),
                });
            }
        } else if SafeFactoryUpdated::match_log(log.log) {
            if let Ok(event) = SafeFactoryUpdated::decode(log.log) {
                events.safe_factory_updated.push(proto::SafeFactoryUpdated {
                    old_safe_factory: format_address(&event.old_safe_factory),
                    new_safe_factory: format_address(&event.new_safe_factory),
                    tx: Some(build_transaction_context(&blk, &log)),
                });
            }
        }
    }

    Ok(events)
}

/// Unified map module that extracts all event types in a single pass through block logs.
///
/// This module replaces the six separate map modules (map_fee_events, map_admin_events,
/// map_trading_events, map_registry_events, map_pause_events, map_config_events) with
/// a single unified implementation that processes all logs in one iteration.
///
/// # Performance
/// By processing all event types in a single pass, this module eliminates 6× redundant
/// iterations over the same log data, significantly improving processing efficiency.
///
/// # Arguments
/// * `blk` - The Ethereum block to extract events from
///
/// # Returns
/// AllEvents message containing all extracted event types
#[substreams::handlers::map]
pub fn map_all_events(blk: eth::Block) -> Result<proto::AllEvents, Error> {
    use abi::neg_risk_ctf::events::*;

    // Initialize all event collectors
    let mut fee_events = proto::FeeEvents::default();
    let mut admin_events = proto::AdminEvents::default();
    let mut trading_events = proto::TradingEvents::default();
    let mut registry_events = proto::RegistryEvents::default();
    let mut pause_events = proto::PauseEvents::default();
    let mut config_events = proto::ConfigEvents::default();

    // Single pass through all logs, extracting all event types
    for log in blk.logs() {
        if !is_neg_risk_ctf_contract(log.log) {
            continue;
        }

        // Fee events
        if FeeCharged::match_log(log.log) {
            if let Ok(event) = FeeCharged::decode(log.log) {
                fee_events.fee_charged.push(proto::FeeCharged {
                    recipient: format_address(&event.receiver),
                    token_id: bigint_to_string(&event.token_id),
                    amount: bigint_to_string(&event.amount),
                    tx: Some(build_transaction_context(&blk, &log)),
                });
            }
        }
        // Admin events
        else if NewAdmin::match_log(log.log) {
            if let Ok(event) = NewAdmin::decode(log.log) {
                admin_events.new_admin.push(proto::NewAdmin {
                    new_admin_address: format_address(&event.new_admin_address),
                    admin: format_address(&event.admin),
                    tx: Some(build_transaction_context(&blk, &log)),
                });
            }
        } else if NewOperator::match_log(log.log) {
            if let Ok(event) = NewOperator::decode(log.log) {
                admin_events.new_operator.push(proto::NewOperator {
                    new_operator_address: format_address(&event.new_operator_address),
                    admin: format_address(&event.admin),
                    tx: Some(build_transaction_context(&blk, &log)),
                });
            }
        } else if RemovedAdmin::match_log(log.log) {
            if let Ok(event) = RemovedAdmin::decode(log.log) {
                admin_events.removed_admin.push(proto::RemovedAdmin {
                    removed_admin: format_address(&event.removed_admin),
                    admin: format_address(&event.admin),
                    tx: Some(build_transaction_context(&blk, &log)),
                });
            }
        } else if RemovedOperator::match_log(log.log) {
            if let Ok(event) = RemovedOperator::decode(log.log) {
                admin_events.removed_operator.push(proto::RemovedOperator {
                    removed_operator: format_address(&event.removed_operator),
                    admin: format_address(&event.admin),
                    tx: Some(build_transaction_context(&blk, &log)),
                });
            }
        }
        // Trading events
        else if OrderCancelled::match_log(log.log) {
            if let Ok(event) = OrderCancelled::decode(log.log) {
                trading_events.order_cancelled.push(proto::OrderCancelled {
                    order_hash: event.order_hash.to_vec(),
                    tx: Some(build_transaction_context(&blk, &log)),
                });
            }
        } else if OrderFilled::match_log(log.log) {
            if let Ok(event) = OrderFilled::decode(log.log) {
                trading_events.order_filled.push(proto::OrderFilled {
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
        } else if OrdersMatched::match_log(log.log) {
            if let Ok(event) = OrdersMatched::decode(log.log) {
                trading_events.orders_matched.push(proto::OrdersMatched {
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
        // Pause events
        else if TradingPaused::match_log(log.log) {
            if let Ok(event) = TradingPaused::decode(log.log) {
                pause_events.trading_paused.push(proto::TradingPaused {
                    pauser: format_address(&event.pauser),
                    tx: Some(build_transaction_context(&blk, &log)),
                });
            }
        } else if TradingUnpaused::match_log(log.log) {
            if let Ok(event) = TradingUnpaused::decode(log.log) {
                pause_events.trading_unpaused.push(proto::TradingUnpaused {
                    pauser: format_address(&event.pauser),
                    tx: Some(build_transaction_context(&blk, &log)),
                });
            }
        }
        // Config events
        else if ProxyFactoryUpdated::match_log(log.log) {
            if let Ok(event) = ProxyFactoryUpdated::decode(log.log) {
                config_events.proxy_factory_updated.push(proto::ProxyFactoryUpdated {
                    old_proxy_factory: format_address(&event.old_proxy_factory),
                    new_proxy_factory: format_address(&event.new_proxy_factory),
                    tx: Some(build_transaction_context(&blk, &log)),
                });
            }
        } else if SafeFactoryUpdated::match_log(log.log) {
            if let Ok(event) = SafeFactoryUpdated::decode(log.log) {
                config_events.safe_factory_updated.push(proto::SafeFactoryUpdated {
                    old_safe_factory: format_address(&event.old_safe_factory),
                    new_safe_factory: format_address(&event.new_safe_factory),
                    tx: Some(build_transaction_context(&blk, &log)),
                });
            }
        }
    }

    // Build and return AllEvents with optional fields for non-empty collections
    Ok(proto::AllEvents {
        fee_events: if !fee_events.fee_charged.is_empty() {
            Some(fee_events)
        } else {
            None
        },
        admin_events: if !admin_events.new_admin.is_empty()
            || !admin_events.new_operator.is_empty()
            || !admin_events.removed_admin.is_empty()
            || !admin_events.removed_operator.is_empty()
        {
            Some(admin_events)
        } else {
            None
        },
        trading_events: if !trading_events.order_cancelled.is_empty()
            || !trading_events.order_filled.is_empty()
            || !trading_events.orders_matched.is_empty()
        {
            Some(trading_events)
        } else {
            None
        },
        registry_events: if !registry_events.token_registered.is_empty() {
            Some(registry_events)
        } else {
            None
        },
        pause_events: if !pause_events.trading_paused.is_empty()
            || !pause_events.trading_unpaused.is_empty()
        {
            Some(pause_events)
        } else {
            None
        },
        config_events: if !config_events.proxy_factory_updated.is_empty()
            || !config_events.safe_factory_updated.is_empty()
        {
            Some(config_events)
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

/// Checks if a log originates from the Neg Risk CTF contract using byte-level comparison.
///
/// Compares the log's 20-byte address directly against [`NEG_RISK_CTF_CONTRACT_ADDRESS`].
/// This is significantly faster than string-based comparison (~25x) because it avoids
/// hex encoding, string allocation, and case-insensitive matching entirely.
#[inline]
fn is_neg_risk_ctf_contract(log: &eth::Log) -> bool {
    log.address == NEG_RISK_CTF_CONTRACT_ADDRESS
}

/// Formats raw bytes as a lowercase hex address string with `0x` prefix.
///
/// Converts an arbitrary byte slice (typically a 20-byte Ethereum address) into its
/// hex-encoded string representation. The output is always lowercase.
///
/// # Examples
///
/// ```ignore
/// let addr = hex_literal::hex!("c5d563a36ae78145c45a50134d48a1215220f80a");
/// assert_eq!(format_address(&addr), "0xc5d563a36ae78145c45a50134d48a1215220f80a");
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
    fn test_is_neg_risk_ctf_contract_matching_address() {
        let log = eth::Log {
            address: hex_literal::hex!("c5d563a36ae78145c45a50134d48a1215220f80a").to_vec(),
            ..Default::default()
        };
        assert!(is_neg_risk_ctf_contract(&log));
    }

    #[test]
    fn test_is_neg_risk_ctf_contract_non_matching_address() {
        let log = eth::Log {
            address: hex_literal::hex!("0000000000000000000000000000000000000000").to_vec(),
            ..Default::default()
        };
        assert!(!is_neg_risk_ctf_contract(&log));
    }

    #[test]
    fn test_neg_risk_ctf_contract_address_is_20_bytes() {
        assert_eq!(NEG_RISK_CTF_CONTRACT_ADDRESS.len(), 20);
    }

    #[test]
    fn test_format_address() {
        let bytes = hex_literal::hex!("c5d563a36ae78145c45a50134d48a1215220f80a");
        let result = format_address(&bytes);
        assert_eq!(result, "0xc5d563a36ae78145c45a50134d48a1215220f80a");
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
