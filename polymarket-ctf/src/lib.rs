// The `#[substreams::handlers::map]` macro generates an `extern "C"` shim that rebuilds
// lazy `&eth::BlockLazyView<'_>` handler inputs from a raw `*mut u8`. The macro drops
// the annotated fn's attributes, so a fn-scoped allow cannot reach the generated
// shim; `not_unsafe_ptr_arg_deref` must be allowed at crate scope. The unsafe deref
// lives entirely in macro-generated code; our handlers are safe.
#![allow(clippy::not_unsafe_ptr_arg_deref)]

#[allow(dead_code, clippy::all)]
pub mod pb;

use substreams::errors::Error;
use substreams_ethereum::pb::eth::v2 as eth;

use pb::polymarket::ctf::v1 as proto;
use polymarket_substreams_common::{
    bigint_to_string, bigint_to_u32, block_logs, format_address, tx_context, BlockLog, BlockLogs,
    CONDITIONAL_TOKENS as CTF_CONTRACT_ADDRESS,
};

/// Map module that extracts CTF-specific events from blocks
#[substreams::handlers::map]
pub fn map_ctf_events(blk: &eth::BlockLazyView<'_>) -> Result<proto::CtfEvents, Error> {
    substreams::skip_empty_output();
    use polymarket_substreams_common::abi::conditional_tokens::events::*;

    let mut events = proto::CtfEvents::default();

    // Iterate over all logs in the block
    // Filter by contract address first (performance optimization)
    let blk = block_logs(blk, is_ctf_contract)?;
    for log in &blk.logs {
        // Try to decode each event type (check match_log first to avoid index errors)
        if ConditionPreparation::match_log(&log.log) {
            if let Ok(event) = ConditionPreparation::decode(&log.log) {
                events
                    .condition_preparation
                    .push(proto::ConditionPreparation {
                        condition_id: event.condition_id.to_vec(),
                        oracle: format_address(&event.oracle),
                        question_id: event.question_id.to_vec(),
                        outcome_slot_count: bigint_to_u32(&event.outcome_slot_count),
                        tx: build_transaction_context(&blk, log).into(),
                    });
            }
        } else if ConditionResolution::match_log(&log.log) {
            if let Ok(event) = ConditionResolution::decode(&log.log) {
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
                        tx: build_transaction_context(&blk, log).into(),
                    });
            }
        } else if PositionSplit::match_log(&log.log) {
            if let Ok(event) = PositionSplit::decode(&log.log) {
                events.position_split.push(proto::PositionSplit {
                    stakeholder: format_address(&event.stakeholder),
                    collateral_token: format_address(&event.collateral_token),
                    parent_collection_id: event.parent_collection_id.to_vec(),
                    condition_id: event.condition_id.to_vec(),
                    partition: event.partition.iter().map(bigint_to_string).collect(),
                    amount: bigint_to_string(&event.amount),
                    tx: build_transaction_context(&blk, log).into(),
                });
            }
        } else if PositionsMerge::match_log(&log.log) {
            if let Ok(event) = PositionsMerge::decode(&log.log) {
                events.positions_merge.push(proto::PositionsMerge {
                    stakeholder: format_address(&event.stakeholder),
                    collateral_token: format_address(&event.collateral_token),
                    parent_collection_id: event.parent_collection_id.to_vec(),
                    condition_id: event.condition_id.to_vec(),
                    partition: event.partition.iter().map(bigint_to_string).collect(),
                    amount: bigint_to_string(&event.amount),
                    tx: build_transaction_context(&blk, log).into(),
                });
            }
        } else if PayoutRedemption::match_log(&log.log) {
            if let Ok(event) = PayoutRedemption::decode(&log.log) {
                events.payout_redemption.push(proto::PayoutRedemption {
                    redeemer: format_address(&event.redeemer),
                    collateral_token: format_address(&event.collateral_token),
                    parent_collection_id: event.parent_collection_id.to_vec(),
                    condition_id: event.condition_id.to_vec(),
                    index_sets: event.index_sets.iter().map(bigint_to_string).collect(),
                    payout: bigint_to_string(&event.payout),
                    tx: build_transaction_context(&blk, log).into(),
                });
            }
        }
        // If none of the decoders match, silently skip (resilient error handling)
    }

    Ok(events)
}

/// Map module that extracts ERC1155 events from blocks
#[substreams::handlers::map]
pub fn map_erc1155_events(blk: &eth::BlockLazyView<'_>) -> Result<proto::Erc1155Events, Error> {
    substreams::skip_empty_output();
    use polymarket_substreams_common::abi::conditional_tokens::events::*;

    let mut events = proto::Erc1155Events::default();

    // Iterate over all logs in the block
    // Filter by contract address first (performance optimization)
    let blk = block_logs(blk, is_ctf_contract)?;
    for log in &blk.logs {
        // Try to decode each ERC1155 event type (check match_log first to avoid index errors)
        if TransferSingle::match_log(&log.log) {
            if let Ok(event) = TransferSingle::decode(&log.log) {
                events.transfer_single.push(proto::TransferSingle {
                    operator: format_address(&event.operator),
                    from: format_address(&event.from),
                    to: format_address(&event.to),
                    id: bigint_to_string(&event.id),
                    value: bigint_to_string(&event.value),
                    tx: build_transaction_context(&blk, log).into(),
                });
            }
        } else if TransferBatch::match_log(&log.log) {
            if let Ok(event) = TransferBatch::decode(&log.log) {
                events.transfer_batch.push(proto::TransferBatch {
                    operator: format_address(&event.operator),
                    from: format_address(&event.from),
                    to: format_address(&event.to),
                    ids: event.ids.iter().map(bigint_to_string).collect(),
                    values: event.values.iter().map(bigint_to_string).collect(),
                    tx: build_transaction_context(&blk, log).into(),
                });
            }
        } else if ApprovalForAll::match_log(&log.log) {
            if let Ok(event) = ApprovalForAll::decode(&log.log) {
                events.approval_for_all.push(proto::ApprovalForAll {
                    account: format_address(&event.owner),
                    operator: format_address(&event.operator),
                    approved: event.approved,
                    tx: build_transaction_context(&blk, log).into(),
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
pub fn map_all_events(blk: &eth::BlockLazyView<'_>) -> Result<proto::AllEvents, Error> {
    substreams::skip_empty_output();
    use polymarket_substreams_common::abi::conditional_tokens::events::*;

    // Initialize all event collectors
    let mut ctf_events = proto::CtfEvents::default();
    let mut erc1155_events = proto::Erc1155Events::default();

    // Single pass through all logs, extracting all event types
    // Filter by contract address first (performance optimization)
    let blk = block_logs(blk, is_ctf_contract)?;
    for log in &blk.logs {
        // CTF events
        if ConditionPreparation::match_log(&log.log) {
            if let Ok(event) = ConditionPreparation::decode(&log.log) {
                ctf_events
                    .condition_preparation
                    .push(proto::ConditionPreparation {
                        condition_id: event.condition_id.to_vec(),
                        oracle: format_address(&event.oracle),
                        question_id: event.question_id.to_vec(),
                        outcome_slot_count: bigint_to_u32(&event.outcome_slot_count),
                        tx: build_transaction_context(&blk, log).into(),
                    });
            }
        } else if ConditionResolution::match_log(&log.log) {
            if let Ok(event) = ConditionResolution::decode(&log.log) {
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
                        tx: build_transaction_context(&blk, log).into(),
                    });
            }
        } else if PositionSplit::match_log(&log.log) {
            if let Ok(event) = PositionSplit::decode(&log.log) {
                ctf_events.position_split.push(proto::PositionSplit {
                    stakeholder: format_address(&event.stakeholder),
                    collateral_token: format_address(&event.collateral_token),
                    parent_collection_id: event.parent_collection_id.to_vec(),
                    condition_id: event.condition_id.to_vec(),
                    partition: event.partition.iter().map(bigint_to_string).collect(),
                    amount: bigint_to_string(&event.amount),
                    tx: build_transaction_context(&blk, log).into(),
                });
            }
        } else if PositionsMerge::match_log(&log.log) {
            if let Ok(event) = PositionsMerge::decode(&log.log) {
                ctf_events.positions_merge.push(proto::PositionsMerge {
                    stakeholder: format_address(&event.stakeholder),
                    collateral_token: format_address(&event.collateral_token),
                    parent_collection_id: event.parent_collection_id.to_vec(),
                    condition_id: event.condition_id.to_vec(),
                    partition: event.partition.iter().map(bigint_to_string).collect(),
                    amount: bigint_to_string(&event.amount),
                    tx: build_transaction_context(&blk, log).into(),
                });
            }
        } else if PayoutRedemption::match_log(&log.log) {
            if let Ok(event) = PayoutRedemption::decode(&log.log) {
                ctf_events.payout_redemption.push(proto::PayoutRedemption {
                    redeemer: format_address(&event.redeemer),
                    collateral_token: format_address(&event.collateral_token),
                    parent_collection_id: event.parent_collection_id.to_vec(),
                    condition_id: event.condition_id.to_vec(),
                    index_sets: event.index_sets.iter().map(bigint_to_string).collect(),
                    payout: bigint_to_string(&event.payout),
                    tx: build_transaction_context(&blk, log).into(),
                });
            }
        }
        // ERC1155 events
        else if TransferSingle::match_log(&log.log) {
            if let Ok(event) = TransferSingle::decode(&log.log) {
                erc1155_events.transfer_single.push(proto::TransferSingle {
                    operator: format_address(&event.operator),
                    from: format_address(&event.from),
                    to: format_address(&event.to),
                    id: bigint_to_string(&event.id),
                    value: bigint_to_string(&event.value),
                    tx: build_transaction_context(&blk, log).into(),
                });
            }
        } else if TransferBatch::match_log(&log.log) {
            if let Ok(event) = TransferBatch::decode(&log.log) {
                erc1155_events.transfer_batch.push(proto::TransferBatch {
                    operator: format_address(&event.operator),
                    from: format_address(&event.from),
                    to: format_address(&event.to),
                    ids: event.ids.iter().map(bigint_to_string).collect(),
                    values: event.values.iter().map(bigint_to_string).collect(),
                    tx: build_transaction_context(&blk, log).into(),
                });
            }
        } else if ApprovalForAll::match_log(&log.log) {
            if let Ok(event) = ApprovalForAll::decode(&log.log) {
                erc1155_events.approval_for_all.push(proto::ApprovalForAll {
                    account: format_address(&event.owner),
                    operator: format_address(&event.operator),
                    approved: event.approved,
                    tx: build_transaction_context(&blk, log).into(),
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
            ctf_events.into()
        } else {
            Default::default()
        },
        erc1155_events: if !erc1155_events.transfer_single.is_empty()
            || !erc1155_events.transfer_batch.is_empty()
            || !erc1155_events.approval_for_all.is_empty()
        {
            erc1155_events.into()
        } else {
            Default::default()
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
fn is_ctf_contract(address: &[u8]) -> bool {
    address == CTF_CONTRACT_ADDRESS
}

/// Builds a [`proto::TransactionContext`] from a kept log via the shared
/// `tx_context` helper, wrapping its package-agnostic fields into this
/// package's proto type.
#[inline]
fn build_transaction_context(blk: &BlockLogs, log: &BlockLog) -> proto::TransactionContext {
    let ctx = tx_context(log.tx_hash, &log.log, blk.number, blk.timestamp);
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
        assert!(is_ctf_contract(&log.address));
    }

    #[test]
    fn test_is_ctf_contract_non_matching() {
        let log = eth::Log {
            address: hex_literal::hex!("0000000000000000000000000000000000000000").to_vec(),
            ..Default::default()
        };
        assert!(!is_ctf_contract(&log.address));
    }

    #[test]
    fn test_is_ctf_contract_empty_address() {
        let log = eth::Log {
            address: vec![],
            ..Default::default()
        };
        assert!(!is_ctf_contract(&log.address));
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
            header: BlockHeader {
                timestamp: buffa_types::google::protobuf::Timestamp {
                    seconds: 1_700_000_000,
                    nanos: 0,
                    ..Default::default()
                }
                .into(),
                ..Default::default()
            }
            .into(),
            transaction_traces: vec![TransactionTrace {
                hash: vec![0xabu8; 32],
                status: eth::TransactionTraceStatus::Succeeded.into(),
                receipt: TransactionReceipt {
                    logs: vec![log],
                    ..Default::default()
                }
                .into(),
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
        let out = polymarket_substreams_common::with_lazy_block(
            &block_with_log(transfer_single_log()),
            |blk| substreams::testing::map!(map_all_events(blk)),
        )
        .expect("fixture block encodes")
        .expect("handler must not err");
        let erc = out
            .erc1155_events
            .as_option()
            .expect("TransferSingle from the CTF contract must land in erc1155_events");
        assert_eq!(erc.transfer_single.len(), 1);
        assert_eq!(
            erc.transfer_single[0].from,
            "0x2222222222222222222222222222222222222222"
        );
        // Must not be misclassified as a CTF (condition/position) event.
        assert!(!out.ctf_events.is_set());
    }

    #[test]
    fn map_all_events_empty_block_ok() {
        let out = polymarket_substreams_common::with_lazy_block(&eth::Block::default(), |blk| {
            substreams::testing::map!(map_all_events(blk))
        })
        .expect("fixture block encodes")
        .expect("empty block must not panic");
        assert!(!out.ctf_events.is_set());
        assert!(!out.erc1155_events.is_set());
    }
}

#[cfg(test)]
mod fixture_tests {
    use super::*;

    /// Real Polygon block 96600000 (captured with `scripts/capture_fixture.sh 96600000`). The
    /// expected counts match `substreams run` of `map_all_events` on the same block.
    #[test]
    fn map_all_events_real_block_96600000() {
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../testdata/polygon_96600000.binpb.b64"
        );
        let block = polymarket_substreams_common::load_block_b64(path).expect("fixture loads");
        assert_eq!(block.number, 96600000);
        let out = polymarket_substreams_common::with_lazy_block(&block, |blk| {
            substreams::testing::map!(map_all_events(blk))
        })
        .expect("fixture block encodes")
        .expect("handler must not err");
        assert_eq!(
            out.ctf_events
                .as_option()
                .map_or(0, |e| e.position_split.len()),
            10,
            "ctf_events.position_split"
        );
        assert_eq!(
            out.ctf_events
                .as_option()
                .map_or(0, |e| e.payout_redemption.len()),
            3,
            "ctf_events.payout_redemption"
        );
        assert_eq!(
            out.erc1155_events
                .as_option()
                .map_or(0, |e| e.transfer_single.len()),
            47,
            "erc1155_events.transfer_single"
        );
        assert_eq!(
            out.erc1155_events
                .as_option()
                .map_or(0, |e| e.transfer_batch.len()),
            27,
            "erc1155_events.transfer_batch"
        );
        assert_eq!(
            out.erc1155_events
                .as_option()
                .map_or(0, |e| e.approval_for_all.len()),
            3,
            "erc1155_events.approval_for_all"
        );
        assert_eq!(
            out.erc1155_events
                .as_option()
                .expect("erc1155 events")
                .transfer_single[0]
                .to,
            "0x45f82472a88f759f8a98174d3cdd3db4a9ee6d79"
        );
    }
}
