pub mod abi;
#[allow(dead_code, clippy::all)]
pub mod pb;

use substreams::errors::Error;
use substreams_ethereum::pb::eth::v2 as eth;

use pb::polymarket::ctf::v1 as proto;
use polymarket_substreams_common::{
    bigint_to_string, bigint_to_u32, build_tx_context, format_address,
    CONDITIONAL_TOKENS as CTF_CONTRACT_ADDRESS,
};

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
                        outcome_slot_count: bigint_to_u32(&event.outcome_slot_count),
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
                        outcome_slot_count: bigint_to_u32(&event.outcome_slot_count),
                        payout_numerators: event
                            .payout_numerators
                            .iter()
                            .map(bigint_to_string)
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
                    partition: event.partition.iter().map(bigint_to_string).collect(),
                    amount: bigint_to_string(&event.amount),
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
                    partition: event.partition.iter().map(bigint_to_string).collect(),
                    amount: bigint_to_string(&event.amount),
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
                    index_sets: event.index_sets.iter().map(bigint_to_string).collect(),
                    payout: bigint_to_string(&event.payout),
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
                    id: bigint_to_string(&event.id),
                    value: bigint_to_string(&event.value),
                    tx: Some(build_transaction_context(&blk, &log)),
                });
            }
        } else if TransferBatch::match_log(log.log) {
            if let Ok(event) = TransferBatch::decode(log.log) {
                events.transfer_batch.push(proto::TransferBatch {
                    operator: format_address(&event.operator),
                    from: format_address(&event.from),
                    to: format_address(&event.to),
                    ids: event.ids.iter().map(bigint_to_string).collect(),
                    values: event.values.iter().map(bigint_to_string).collect(),
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
                        outcome_slot_count: bigint_to_u32(&event.outcome_slot_count),
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
                        outcome_slot_count: bigint_to_u32(&event.outcome_slot_count),
                        payout_numerators: event
                            .payout_numerators
                            .iter()
                            .map(bigint_to_string)
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
                    partition: event.partition.iter().map(bigint_to_string).collect(),
                    amount: bigint_to_string(&event.amount),
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
                    partition: event.partition.iter().map(bigint_to_string).collect(),
                    amount: bigint_to_string(&event.amount),
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
                    index_sets: event.index_sets.iter().map(bigint_to_string).collect(),
                    payout: bigint_to_string(&event.payout),
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
                    id: bigint_to_string(&event.id),
                    value: bigint_to_string(&event.value),
                    tx: Some(build_transaction_context(&blk, &log)),
                });
            }
        } else if TransferBatch::match_log(log.log) {
            if let Ok(event) = TransferBatch::decode(log.log) {
                erc1155_events.transfer_batch.push(proto::TransferBatch {
                    operator: format_address(&event.operator),
                    from: format_address(&event.from),
                    to: format_address(&event.to),
                    ids: event.ids.iter().map(bigint_to_string).collect(),
                    values: event.values.iter().map(bigint_to_string).collect(),
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

/// Builds a [`proto::TransactionContext`] from block and log data via the shared
/// `build_tx_context` helper, wrapping its package-agnostic fields into this
/// package's proto type.
#[inline]
fn build_transaction_context(
    blk: &eth::Block,
    log: &substreams_ethereum::block_view::LogView,
) -> proto::TransactionContext {
    let ctx = build_tx_context(blk, log);
    proto::TransactionContext {
        tx_hash: ctx.tx_hash,
        log_index: ctx.log_index,
        block_number: ctx.block_number,
        timestamp: ctx.timestamp,
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

#[cfg(test)]
mod handler_tests {
    use super::*;
    use substreams_ethereum::pb::eth::v2 as eth;

    fn block_with_log(log: eth::Log) -> eth::Block {
        use substreams_ethereum::pb::eth::v2::{BlockHeader, TransactionReceipt, TransactionTrace};
        eth::Block {
            number: 42,
            header: Some(BlockHeader {
                timestamp: Some(prost_types::Timestamp {
                    seconds: 1_700_000_000,
                    nanos: 0,
                }),
                ..Default::default()
            }),
            transaction_traces: vec![TransactionTrace {
                hash: vec![0xabu8; 32],
                status: 1,
                receipt: Some(TransactionReceipt {
                    logs: vec![log],
                    ..Default::default()
                }),
                ..Default::default()
            }],
            ..Default::default()
        }
    }

    fn topic_addr(addr: &[u8; 20]) -> Vec<u8> {
        let mut t = vec![0u8; 12];
        t.extend_from_slice(addr);
        t
    }

    fn transfer_single_log() -> eth::Log {
        let topic0 =
            hex_literal::hex!("c3d58168c5ae7397731d063d5bbf3d657854427343f4c083240f7aacaa2d0f62")
                .to_vec();
        let operator = [0x11u8; 20];
        let from = [0x22u8; 20];
        let to = [0x33u8; 20];
        eth::Log {
            address: CTF_CONTRACT_ADDRESS.to_vec(),
            topics: vec![
                topic0,
                topic_addr(&operator),
                topic_addr(&from),
                topic_addr(&to),
            ],
            data: vec![0u8; 64], // id (uint256) || value (uint256)
            ..Default::default()
        }
    }

    #[test]
    fn map_all_events_classifies_transfer_single() {
        let out =
            __impl_map_all_events(block_with_log(transfer_single_log())).expect("handler must not err");
        let erc = out
            .erc1155_events
            .expect("TransferSingle from the CTF contract must land in erc1155_events");
        assert_eq!(erc.transfer_single.len(), 1);
        assert_eq!(
            erc.transfer_single[0].from,
            "0x2222222222222222222222222222222222222222"
        );
        // Must not be misclassified as a CTF (condition/position) event.
        assert!(out.ctf_events.is_none());
    }

    #[test]
    fn map_all_events_empty_block_ok() {
        let out = __impl_map_all_events(eth::Block::default()).expect("empty block must not panic");
        assert!(out.ctf_events.is_none());
        assert!(out.erc1155_events.is_none());
    }
}
