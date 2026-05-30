pub mod abi;
#[allow(dead_code, clippy::all)]
pub mod pb;

use substreams::errors::Error;
use substreams_ethereum::pb::eth::v2 as eth;

use pb::polymarket::ctf::v1 as proto;

/// Ethereum address of the Polymarket Conditional Tokens Framework (CTF) contract on Polygon.
///
/// This is stored as a raw 20-byte array to enable direct byte-level comparison against
/// log addresses, avoiding the overhead of string parsing and heap allocation on every log entry.
///
/// See: <https://polygonscan.com/address/0x4D97DCd97eC945f40cF65F87097ACe5EA0476045>
const CTF_CONTRACT_ADDRESS: [u8; 20] =
    hex_literal::hex!("4D97DCd97eC945f40cF65F87097ACe5EA0476045");

/// Map module that extracts CTF-specific events from blocks
#[substreams::handlers::map]
pub fn map_ctf_events(blk: eth::Block) -> Result<proto::CtfEvents, Error> {
    use abi::conditional_tokens::events::*;

    let mut events = proto::CtfEvents::default();

    // Iterate over all logs in the block
    for log in blk.logs() {
        // Filter by contract address first (performance optimization)
        if !is_ctf_contract(log.log) {
            continue;
        }

        // Try to decode each event type (check match_log first to avoid index errors)
        if ConditionPreparation::match_log(log.log) {
            if let Ok(event) = ConditionPreparation::decode(log.log) {
                events
                    .condition_preparation
                    .push(proto::ConditionPreparation {
                        condition_id: event.condition_id.to_vec(),
                        oracle: format_address(&event.oracle),
                        question_id: event.question_id.to_vec(),
                        outcome_slot_count: event
                            .outcome_slot_count
                            .to_string()
                            .parse()
                            .unwrap_or(0),
                        tx: Some(build_transaction_context(&blk, &log)),
                    });
            }
        } else if ConditionResolution::match_log(log.log) {
            if let Ok(event) = ConditionResolution::decode(log.log) {
                events
                    .condition_resolution
                    .push(proto::ConditionResolution {
                        condition_id: event.condition_id.to_vec(),
                        oracle: format_address(&event.oracle),
                        question_id: event.question_id.to_vec(),
                        outcome_slot_count: event
                            .outcome_slot_count
                            .to_string()
                            .parse()
                            .unwrap_or(0),
                        payout_numerators: event
                            .payout_numerators
                            .iter()
                            .map(|n| n.to_string())
                            .collect(),
                        tx: Some(build_transaction_context(&blk, &log)),
                    });
            }
        } else if PositionSplit::match_log(log.log) {
            if let Ok(event) = PositionSplit::decode(log.log) {
                events.position_split.push(proto::PositionSplit {
                    stakeholder: format_address(&event.stakeholder),
                    collateral_token: format_address(&event.collateral_token),
                    parent_collection_id: event.parent_collection_id.to_vec(),
                    condition_id: event.condition_id.to_vec(),
                    partition: event.partition.iter().map(|n| n.to_string()).collect(),
                    amount: event.amount.to_string(),
                    tx: Some(build_transaction_context(&blk, &log)),
                });
            }
        } else if PositionsMerge::match_log(log.log) {
            if let Ok(event) = PositionsMerge::decode(log.log) {
                events.positions_merge.push(proto::PositionsMerge {
                    stakeholder: format_address(&event.stakeholder),
                    collateral_token: format_address(&event.collateral_token),
                    parent_collection_id: event.parent_collection_id.to_vec(),
                    condition_id: event.condition_id.to_vec(),
                    partition: event.partition.iter().map(|n| n.to_string()).collect(),
                    amount: event.amount.to_string(),
                    tx: Some(build_transaction_context(&blk, &log)),
                });
            }
        } else if PayoutRedemption::match_log(log.log) {
            if let Ok(event) = PayoutRedemption::decode(log.log) {
                events.payout_redemption.push(proto::PayoutRedemption {
                    redeemer: format_address(&event.redeemer),
                    collateral_token: format_address(&event.collateral_token),
                    parent_collection_id: event.parent_collection_id.to_vec(),
                    condition_id: event.condition_id.to_vec(),
                    index_sets: event.index_sets.iter().map(|n| n.to_string()).collect(),
                    payout: event.payout.to_string(),
                    tx: Some(build_transaction_context(&blk, &log)),
                });
            }
        }
        // If none of the decoders match, silently skip (resilient error handling)
    }

    Ok(events)
}

/// Map module that extracts ERC1155 events from blocks
#[substreams::handlers::map]
pub fn map_erc1155_events(blk: eth::Block) -> Result<proto::Erc1155Events, Error> {
    use abi::conditional_tokens::events::*;

    let mut events = proto::Erc1155Events::default();

    // Iterate over all logs in the block
    for log in blk.logs() {
        // Filter by contract address first (performance optimization)
        if !is_ctf_contract(log.log) {
            continue;
        }

        // Try to decode each ERC1155 event type (check match_log first to avoid index errors)
        if TransferSingle::match_log(log.log) {
            if let Ok(event) = TransferSingle::decode(log.log) {
                events.transfer_single.push(proto::TransferSingle {
                    operator: format_address(&event.operator),
                    from: format_address(&event.from),
                    to: format_address(&event.to),
                    id: event.id.to_string(),
                    value: event.value.to_string(),
                    tx: Some(build_transaction_context(&blk, &log)),
                });
            }
        } else if TransferBatch::match_log(log.log) {
            if let Ok(event) = TransferBatch::decode(log.log) {
                events.transfer_batch.push(proto::TransferBatch {
                    operator: format_address(&event.operator),
                    from: format_address(&event.from),
                    to: format_address(&event.to),
                    ids: event.ids.iter().map(|id| id.to_string()).collect(),
                    values: event.values.iter().map(|v| v.to_string()).collect(),
                    tx: Some(build_transaction_context(&blk, &log)),
                });
            }
        } else if ApprovalForAll::match_log(log.log) {
            if let Ok(event) = ApprovalForAll::decode(log.log) {
                events.approval_for_all.push(proto::ApprovalForAll {
                    account: format_address(&event.owner),
                    operator: format_address(&event.operator),
                    approved: event.approved,
                    tx: Some(build_transaction_context(&blk, &log)),
                });
            }
        }
        // If none of the decoders match, silently skip (resilient error handling)
    }

    Ok(events)
}

/// Unified map module that extracts all event types in a single pass through block logs.
///
/// This module processes both CTF-specific and ERC1155 events in one iteration,
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
    use abi::conditional_tokens::events::*;

    // Initialize all event collectors
    let mut ctf_events = proto::CtfEvents::default();
    let mut erc1155_events = proto::Erc1155Events::default();

    // Single pass through all logs, extracting all event types
    for log in blk.logs() {
        // Filter by contract address first (performance optimization)
        if !is_ctf_contract(log.log) {
            continue;
        }

        // CTF events
        if ConditionPreparation::match_log(log.log) {
            if let Ok(event) = ConditionPreparation::decode(log.log) {
                ctf_events
                    .condition_preparation
                    .push(proto::ConditionPreparation {
                        condition_id: event.condition_id.to_vec(),
                        oracle: format_address(&event.oracle),
                        question_id: event.question_id.to_vec(),
                        outcome_slot_count: event
                            .outcome_slot_count
                            .to_string()
                            .parse()
                            .unwrap_or(0),
                        tx: Some(build_transaction_context(&blk, &log)),
                    });
            }
        } else if ConditionResolution::match_log(log.log) {
            if let Ok(event) = ConditionResolution::decode(log.log) {
                ctf_events
                    .condition_resolution
                    .push(proto::ConditionResolution {
                        condition_id: event.condition_id.to_vec(),
                        oracle: format_address(&event.oracle),
                        question_id: event.question_id.to_vec(),
                        outcome_slot_count: event
                            .outcome_slot_count
                            .to_string()
                            .parse()
                            .unwrap_or(0),
                        payout_numerators: event
                            .payout_numerators
                            .iter()
                            .map(|n| n.to_string())
                            .collect(),
                        tx: Some(build_transaction_context(&blk, &log)),
                    });
            }
        } else if PositionSplit::match_log(log.log) {
            if let Ok(event) = PositionSplit::decode(log.log) {
                ctf_events.position_split.push(proto::PositionSplit {
                    stakeholder: format_address(&event.stakeholder),
                    collateral_token: format_address(&event.collateral_token),
                    parent_collection_id: event.parent_collection_id.to_vec(),
                    condition_id: event.condition_id.to_vec(),
                    partition: event.partition.iter().map(|n| n.to_string()).collect(),
                    amount: event.amount.to_string(),
                    tx: Some(build_transaction_context(&blk, &log)),
                });
            }
        } else if PositionsMerge::match_log(log.log) {
            if let Ok(event) = PositionsMerge::decode(log.log) {
                ctf_events.positions_merge.push(proto::PositionsMerge {
                    stakeholder: format_address(&event.stakeholder),
                    collateral_token: format_address(&event.collateral_token),
                    parent_collection_id: event.parent_collection_id.to_vec(),
                    condition_id: event.condition_id.to_vec(),
                    partition: event.partition.iter().map(|n| n.to_string()).collect(),
                    amount: event.amount.to_string(),
                    tx: Some(build_transaction_context(&blk, &log)),
                });
            }
        } else if PayoutRedemption::match_log(log.log) {
            if let Ok(event) = PayoutRedemption::decode(log.log) {
                ctf_events.payout_redemption.push(proto::PayoutRedemption {
                    redeemer: format_address(&event.redeemer),
                    collateral_token: format_address(&event.collateral_token),
                    parent_collection_id: event.parent_collection_id.to_vec(),
                    condition_id: event.condition_id.to_vec(),
                    index_sets: event.index_sets.iter().map(|n| n.to_string()).collect(),
                    payout: event.payout.to_string(),
                    tx: Some(build_transaction_context(&blk, &log)),
                });
            }
        }
        // ERC1155 events
        else if TransferSingle::match_log(log.log) {
            if let Ok(event) = TransferSingle::decode(log.log) {
                erc1155_events.transfer_single.push(proto::TransferSingle {
                    operator: format_address(&event.operator),
                    from: format_address(&event.from),
                    to: format_address(&event.to),
                    id: event.id.to_string(),
                    value: event.value.to_string(),
                    tx: Some(build_transaction_context(&blk, &log)),
                });
            }
        } else if TransferBatch::match_log(log.log) {
            if let Ok(event) = TransferBatch::decode(log.log) {
                erc1155_events.transfer_batch.push(proto::TransferBatch {
                    operator: format_address(&event.operator),
                    from: format_address(&event.from),
                    to: format_address(&event.to),
                    ids: event.ids.iter().map(|id| id.to_string()).collect(),
                    values: event.values.iter().map(|v| v.to_string()).collect(),
                    tx: Some(build_transaction_context(&blk, &log)),
                });
            }
        } else if ApprovalForAll::match_log(log.log) {
            if let Ok(event) = ApprovalForAll::decode(log.log) {
                erc1155_events.approval_for_all.push(proto::ApprovalForAll {
                    account: format_address(&event.owner),
                    operator: format_address(&event.operator),
                    approved: event.approved,
                    tx: Some(build_transaction_context(&blk, &log)),
                });
            }
        }
        // If none of the decoders match, silently skip (resilient error handling)
    }

    // Build and return AllEvents with optional fields for non-empty collections
    Ok(proto::AllEvents {
        ctf_events: if !ctf_events.condition_preparation.is_empty()
            || !ctf_events.condition_resolution.is_empty()
            || !ctf_events.position_split.is_empty()
            || !ctf_events.positions_merge.is_empty()
            || !ctf_events.payout_redemption.is_empty()
        {
            Some(ctf_events)
        } else {
            None
        },
        erc1155_events: if !erc1155_events.transfer_single.is_empty()
            || !erc1155_events.transfer_batch.is_empty()
            || !erc1155_events.approval_for_all.is_empty()
        {
            Some(erc1155_events)
        } else {
            None
        },
    })
}

// Helper functions

/// Checks if a log originates from the CTF contract using byte-level comparison.
///
/// Compares the log's 20-byte address directly against [`CTF_CONTRACT_ADDRESS`].
/// This is significantly faster than string-based comparison (~25x) because it avoids
/// hex encoding, string allocation, and case-insensitive matching entirely.
#[inline]
fn is_ctf_contract(log: &eth::Log) -> bool {
    log.address == CTF_CONTRACT_ADDRESS
}

/// Formats raw bytes as a lowercase hex address string with `0x` prefix.
///
/// Converts an arbitrary byte slice (typically a 20-byte Ethereum address) into its
/// hex-encoded string representation. The output is always lowercase.
///
/// # Examples
///
/// ```ignore
/// let addr = hex_literal::hex!("4D97DCd97eC945f40cF65F87097ACe5EA0476045");
/// assert_eq!(format_address(&addr), "0x4d97dcd97ec945f40cf65f87097ace5ea0476045");
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
fn build_transaction_context(
    blk: &eth::Block,
    log: &substreams_ethereum::block_view::LogView,
) -> proto::TransactionContext {
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

    #[test]
    fn test_ctf_contract_address_bytes() {
        // Verify the constant matches the expected address
        let expected = hex_literal::hex!("4D97DCd97eC945f40cF65F87097ACe5EA0476045");
        assert_eq!(CTF_CONTRACT_ADDRESS, expected);
        assert_eq!(CTF_CONTRACT_ADDRESS.len(), 20);
    }

    #[test]
    fn test_is_ctf_contract_matching() {
        let log = eth::Log {
            address: hex_literal::hex!("4D97DCd97eC945f40cF65F87097ACe5EA0476045").to_vec(),
            ..Default::default()
        };
        assert!(is_ctf_contract(&log));
    }

    #[test]
    fn test_is_ctf_contract_non_matching() {
        let log = eth::Log {
            address: hex_literal::hex!("0000000000000000000000000000000000000000").to_vec(),
            ..Default::default()
        };
        assert!(!is_ctf_contract(&log));
    }

    #[test]
    fn test_is_ctf_contract_empty_address() {
        let log = eth::Log {
            address: vec![],
            ..Default::default()
        };
        assert!(!is_ctf_contract(&log));
    }

    #[test]
    fn test_format_address() {
        let bytes = hex_literal::hex!("4D97DCd97eC945f40cF65F87097ACe5EA0476045");
        let result = format_address(&bytes);
        assert_eq!(result, "0x4d97dcd97ec945f40cf65f87097ace5ea0476045");
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
