// The `#[substreams::handlers::map]` macro generates an `extern "C"` shim that rebuilds
// lazy `&eth::BlockLazyView<'_>` handler inputs from a raw `*mut u8`. The macro drops
// the annotated fn's attributes, so a fn-scoped allow cannot reach the generated
// shim; `not_unsafe_ptr_arg_deref` must be allowed at crate scope. The unsafe deref
// lives entirely in macro-generated code; our handlers are safe.
#![allow(clippy::not_unsafe_ptr_arg_deref)]

pub mod abi;
#[allow(dead_code, clippy::all)]
pub mod pb;

use substreams::errors::Error;
use substreams_ethereum::pb::eth::v2 as eth;

use pb::polymarket::resolution::v1 as proto;
use polymarket_substreams_common::{
    bigint_to_string, bigint_to_u64, block_logs, format_address, tx_context, BlockLog, BlockLogs,
};

const UMA_ORACLE_V2_ADDRESS: [u8; 20] =
    hex_literal::hex!("ee3afe347d5c74317041e2618c49534daf887c24");

const UMA_ORACLE_V3_ADDRESS: [u8; 20] =
    hex_literal::hex!("5953f2538f613e05baed8a5aefa8e6622467ad3d");

const CTF_ADAPTER_V2_ADDRESS: [u8; 20] =
    hex_literal::hex!("6a9d222616c90fca5754cd1333cfd9b7fb6a4f74");

const CTF_ADAPTER_V3_ADDRESS: [u8; 20] =
    hex_literal::hex!("2f5e3684cb1f318ec51b00edba38d79ac2c0aa9d");

/// Protocol V2 BinaryModule (proxy).
const BINARY_MODULE_ADDRESS: [u8; 20] =
    hex_literal::hex!("1000008dd9001b968442c1000017eae6e0da00ba");

/// Protocol V2 NegRiskModule (proxy).
const NEG_RISK_MODULE_ADDRESS: [u8; 20] =
    hex_literal::hex!("200000900045e3b6259600682756002200028933");

fn is_module_address(addr: &[u8]) -> bool {
    addr == BINARY_MODULE_ADDRESS || addr == NEG_RISK_MODULE_ADDRESS
}

fn module_name(addr: &[u8]) -> &'static str {
    if addr == BINARY_MODULE_ADDRESS {
        "binary"
    } else {
        "neg_risk"
    }
}

fn is_adapter_address(addr: &[u8]) -> bool {
    addr == CTF_ADAPTER_V2_ADDRESS || addr == CTF_ADAPTER_V3_ADDRESS
}

fn adapter_source(addr: &[u8]) -> &'static str {
    if addr == CTF_ADAPTER_V2_ADDRESS {
        "adapter_v2"
    } else {
        "adapter_v3"
    }
}

fn is_resolution_address(addr: &[u8]) -> bool {
    addr == UMA_ORACLE_V2_ADDRESS
        || addr == UMA_ORACLE_V3_ADDRESS
        || is_adapter_address(addr)
        || is_module_address(addr)
}

fn to_proto_tx(blk: &BlockLogs, log: &BlockLog) -> proto::TransactionContext {
    let ctx = tx_context(log.tx_hash, &log.log, blk.number, blk.timestamp);
    proto::TransactionContext {
        tx_hash: ctx.tx_hash,
        log_index: ctx.log_index,
        block_number: ctx.block_number,
        timestamp: ctx.timestamp,
    }
}

/// All resolution events extracted from one block in a single pass.
///
/// Oracle-derived and adapter-derived dispute alerts are kept in separate vecs so
/// callers can concatenate them oracle-first — preserving the exact ordering of
/// the previous two-pass (decode_oracle_events + decode_adapter_events)
/// implementation, where all oracle alerts preceded all adapter alerts.
#[derive(Default)]
struct DecodedBlock {
    oracle: proto::OracleEvents,
    adapter: proto::AdapterEvents,
    oracle_alerts: Vec<proto::DisputeAlert>,
    adapter_alerts: Vec<proto::DisputeAlert>,
    modules: proto::ModuleEvents,
}

/// Single pass over `blk.logs()`, classifying each log by its emitting contract
/// via an if/else-if chain (matching the `map_all_events` convention in the other
/// packages). The transaction context is built once per event and shared between
/// the event struct and its dispute alert via `tx.clone()`.
fn decode_block(blk: &eth::BlockLazyView) -> Result<DecodedBlock, Error> {
    use abi::optimistic_oracle_v2::events as v2;
    use abi::optimistic_oracle_v3::events as v3;
    use abi::uma_ctf_adapter::events::*;

    let mut d = DecodedBlock::default();

    let blk = &block_logs(blk, is_resolution_address)?;
    for log in &blk.logs {
        if log.log.address == UMA_ORACLE_V2_ADDRESS {
            if v2::ProposePrice::match_log(&log.log) {
                if let Ok(e) = v2::ProposePrice::decode(&log.log) {
                    let tx = to_proto_tx(blk, log);
                    d.oracle.proposals.push(proto::ResolutionProposal {
                        oracle_version: "v2".into(),
                        proposer: format_address(&e.proposer),
                        identifier: e.identifier.to_vec(),
                        question_data: e.ancillary_data,
                        proposed_price: bigint_to_string(&e.proposed_price),
                        currency: format_address(&e.currency),
                        expiration: bigint_to_u64(&e.expiration_timestamp),
                        requester: format_address(&e.requester),
                        assertion_id: Vec::new(),
                        bond: String::new(),
                        tx: tx.into(),
                    });
                }
            } else if v2::DisputePrice::match_log(&log.log) {
                if let Ok(e) = v2::DisputePrice::decode(&log.log) {
                    let tx = to_proto_tx(blk, log);
                    d.oracle.disputes.push(proto::ResolutionDispute {
                        oracle_version: "v2".into(),
                        disputer: format_address(&e.disputer),
                        identifier: e.identifier.to_vec(),
                        assertion_id: Vec::new(),
                        question_data: e.ancillary_data.clone(),
                        proposer: format_address(&e.proposer),
                        proposed_price: bigint_to_string(&e.proposed_price),
                        tx: tx.clone().into(),
                    });
                    d.oracle_alerts.push(proto::DisputeAlert {
                        source: "oracle_v2".into(),
                        alert_type: "dispute".into(),
                        disputer: format_address(&e.disputer),
                        identifier: e.identifier.to_vec(),
                        ancillary_data: e.ancillary_data,
                        payouts: Vec::new(),
                        tx: tx.into(),
                    });
                }
            } else if v2::Settle::match_log(&log.log) {
                if let Ok(e) = v2::Settle::decode(&log.log) {
                    let tx = to_proto_tx(blk, log);
                    d.oracle.settlements.push(proto::ResolutionSettlement {
                        oracle_version: "v2".into(),
                        price: bigint_to_string(&e.price),
                        disputed: !e.disputer.iter().all(|&b| b == 0),
                        proposer: format_address(&e.proposer),
                        disputer: format_address(&e.disputer),
                        identifier: e.identifier.to_vec(),
                        assertion_id: Vec::new(),
                        payout: bigint_to_string(&e.payout),
                        question_data: e.ancillary_data,
                        tx: tx.into(),
                    });
                }
            }
        } else if log.log.address == UMA_ORACLE_V3_ADDRESS {
            if v3::AssertionMade::match_log(&log.log) {
                if let Ok(e) = v3::AssertionMade::decode(&log.log) {
                    let tx = to_proto_tx(blk, log);
                    d.oracle.proposals.push(proto::ResolutionProposal {
                        oracle_version: "v3".into(),
                        proposer: format_address(&e.asserter),
                        identifier: e.identifier.to_vec(),
                        question_data: e.claim,
                        proposed_price: String::new(),
                        currency: format_address(&e.currency),
                        expiration: bigint_to_u64(&e.expiration_time),
                        requester: String::new(),
                        assertion_id: e.assertion_id.to_vec(),
                        bond: bigint_to_string(&e.bond),
                        tx: tx.into(),
                    });
                }
            } else if v3::AssertionDisputed::match_log(&log.log) {
                if let Ok(e) = v3::AssertionDisputed::decode(&log.log) {
                    let tx = to_proto_tx(blk, log);
                    d.oracle.disputes.push(proto::ResolutionDispute {
                        oracle_version: "v3".into(),
                        disputer: format_address(&e.disputer),
                        identifier: Vec::new(),
                        assertion_id: e.assertion_id.to_vec(),
                        question_data: Vec::new(),
                        proposer: String::new(),
                        proposed_price: String::new(),
                        tx: tx.clone().into(),
                    });
                    d.oracle_alerts.push(proto::DisputeAlert {
                        source: "oracle_v3".into(),
                        alert_type: "dispute".into(),
                        disputer: format_address(&e.disputer),
                        identifier: e.assertion_id.to_vec(),
                        ancillary_data: Vec::new(),
                        payouts: Vec::new(),
                        tx: tx.into(),
                    });
                }
            } else if v3::AssertionSettled::match_log(&log.log) {
                if let Ok(e) = v3::AssertionSettled::decode(&log.log) {
                    let tx = to_proto_tx(blk, log);
                    d.oracle.settlements.push(proto::ResolutionSettlement {
                        oracle_version: "v3".into(),
                        price: if e.settlement_resolution { "1" } else { "0" }.into(),
                        disputed: e.disputed,
                        proposer: String::new(),
                        disputer: String::new(),
                        identifier: Vec::new(),
                        assertion_id: e.assertion_id.to_vec(),
                        payout: String::new(),
                        question_data: Vec::new(),
                        tx: tx.into(),
                    });
                }
            }
        } else if is_module_address(log.log.address) {
            use abi::protocol_v2_module::events::ConditionResolved;
            if ConditionResolved::match_log(&log.log) {
                if let Ok(e) = ConditionResolved::decode(&log.log) {
                    d.modules
                        .condition_resolved
                        .push(proto::ModuleConditionResolved {
                            // 0x-prefixed hex; `format_address` hex-encodes any byte slice.
                            condition_id: format_address(&e.condition_id),
                            payouts: e.result.iter().map(bigint_to_string).collect(),
                            module: module_name(log.log.address).into(),
                            tx: to_proto_tx(blk, log).into(),
                        });
                }
            }
        } else if is_adapter_address(log.log.address) {
            let source = adapter_source(log.log.address);

            if QuestionInitialized::match_log(&log.log) {
                if let Ok(e) = QuestionInitialized::decode(&log.log) {
                    let tx = to_proto_tx(blk, log);
                    d.adapter
                        .question_initialized
                        .push(proto::QuestionInitialized {
                            question_id: e.question_id.to_vec(),
                            request_timestamp: bigint_to_string(&e.request_timestamp),
                            creator: format_address(&e.creator),
                            ancillary_data: e.ancillary_data,
                            reward_token: format_address(&e.reward_token),
                            reward: bigint_to_string(&e.reward),
                            proposal_bond: bigint_to_string(&e.proposal_bond),
                            tx: tx.into(),
                        });
                }
            } else if QuestionResolved::match_log(&log.log) {
                if let Ok(e) = QuestionResolved::decode(&log.log) {
                    let tx = to_proto_tx(blk, log);
                    d.adapter.question_resolved.push(proto::QuestionResolved {
                        question_id: e.question_id.to_vec(),
                        settled_price: bigint_to_string(&e.settled_price),
                        payouts: e.payouts.iter().map(bigint_to_string).collect(),
                        tx: tx.into(),
                    });
                }
            } else if QuestionEmergencyResolved::match_log(&log.log) {
                if let Ok(e) = QuestionEmergencyResolved::decode(&log.log) {
                    let tx = to_proto_tx(blk, log);
                    let payouts: Vec<String> = e.payouts.iter().map(bigint_to_string).collect();
                    d.adapter
                        .question_emergency_resolved
                        .push(proto::QuestionEmergencyResolved {
                            question_id: e.question_id.to_vec(),
                            payouts: payouts.clone(),
                            tx: tx.clone().into(),
                        });
                    d.adapter_alerts.push(proto::DisputeAlert {
                        source: source.into(),
                        alert_type: "emergency_resolve".into(),
                        disputer: String::new(),
                        identifier: e.question_id.to_vec(),
                        ancillary_data: Vec::new(),
                        payouts,
                        tx: tx.into(),
                    });
                }
            } else if QuestionFlagged::match_log(&log.log) {
                if let Ok(e) = QuestionFlagged::decode(&log.log) {
                    let tx = to_proto_tx(blk, log);
                    d.adapter.question_flagged.push(proto::QuestionFlagged {
                        question_id: e.question_id.to_vec(),
                        tx: tx.clone().into(),
                    });
                    d.adapter_alerts.push(proto::DisputeAlert {
                        source: source.into(),
                        alert_type: "flag".into(),
                        disputer: String::new(),
                        identifier: e.question_id.to_vec(),
                        ancillary_data: Vec::new(),
                        payouts: Vec::new(),
                        tx: tx.into(),
                    });
                }
            } else if QuestionUnflagged::match_log(&log.log) {
                if let Ok(e) = QuestionUnflagged::decode(&log.log) {
                    let tx = to_proto_tx(blk, log);
                    d.adapter.question_unflagged.push(proto::QuestionUnflagged {
                        question_id: e.question_id.to_vec(),
                        tx: tx.into(),
                    });
                }
            } else if QuestionPaused::match_log(&log.log) {
                if let Ok(e) = QuestionPaused::decode(&log.log) {
                    let tx = to_proto_tx(blk, log);
                    d.adapter.question_paused.push(proto::QuestionPaused {
                        question_id: e.question_id.to_vec(),
                        tx: tx.clone().into(),
                    });
                    d.adapter_alerts.push(proto::DisputeAlert {
                        source: source.into(),
                        alert_type: "pause".into(),
                        disputer: String::new(),
                        identifier: e.question_id.to_vec(),
                        ancillary_data: Vec::new(),
                        payouts: Vec::new(),
                        tx: tx.into(),
                    });
                }
            } else if QuestionUnpaused::match_log(&log.log) {
                if let Ok(e) = QuestionUnpaused::decode(&log.log) {
                    let tx = to_proto_tx(blk, log);
                    d.adapter.question_unpaused.push(proto::QuestionUnpaused {
                        question_id: e.question_id.to_vec(),
                        tx: tx.into(),
                    });
                }
            } else if QuestionReset::match_log(&log.log) {
                if let Ok(e) = QuestionReset::decode(&log.log) {
                    let tx = to_proto_tx(blk, log);
                    d.adapter.question_reset.push(proto::QuestionReset {
                        question_id: e.question_id.to_vec(),
                        tx: tx.clone().into(),
                    });
                    d.adapter_alerts.push(proto::DisputeAlert {
                        source: source.into(),
                        alert_type: "reset".into(),
                        disputer: String::new(),
                        identifier: e.question_id.to_vec(),
                        ancillary_data: Vec::new(),
                        payouts: Vec::new(),
                        tx: tx.into(),
                    });
                }
            } else if AncillaryDataUpdated::match_log(&log.log) {
                if let Ok(e) = AncillaryDataUpdated::decode(&log.log) {
                    let tx = to_proto_tx(blk, log);
                    d.adapter
                        .ancillary_data_updated
                        .push(proto::AncillaryDataUpdated {
                            question_id: e.question_id.to_vec(),
                            owner: format_address(&e.owner),
                            update_data: e.update,
                            tx: tx.into(),
                        });
                }
            } else if NewAdmin::match_log(&log.log) {
                if let Ok(e) = NewAdmin::decode(&log.log) {
                    let tx = to_proto_tx(blk, log);
                    d.adapter.admin_changes.push(proto::AdapterAdmin {
                        admin: format_address(&e.admin),
                        target_admin: format_address(&e.new_admin_address),
                        is_addition: true,
                        tx: tx.into(),
                    });
                }
            } else if RemovedAdmin::match_log(&log.log) {
                if let Ok(e) = RemovedAdmin::decode(&log.log) {
                    let tx = to_proto_tx(blk, log);
                    d.adapter.admin_changes.push(proto::AdapterAdmin {
                        admin: format_address(&e.admin),
                        target_admin: format_address(&e.removed_admin),
                        is_addition: false,
                        tx: tx.into(),
                    });
                }
            }
        }
    }

    Ok(d)
}

fn has_oracle_events(o: &proto::OracleEvents) -> bool {
    !o.proposals.is_empty() || !o.disputes.is_empty() || !o.settlements.is_empty()
}

fn has_adapter_events(a: &proto::AdapterEvents) -> bool {
    !a.question_initialized.is_empty()
        || !a.question_resolved.is_empty()
        || !a.question_emergency_resolved.is_empty()
        || !a.question_flagged.is_empty()
        || !a.question_unflagged.is_empty()
        || !a.question_paused.is_empty()
        || !a.question_unpaused.is_empty()
        || !a.question_reset.is_empty()
        || !a.ancillary_data_updated.is_empty()
        || !a.admin_changes.is_empty()
}

#[substreams::handlers::map]
pub fn map_oracle_events(blk: &eth::BlockLazyView<'_>) -> Result<proto::OracleEvents, Error> {
    substreams::skip_empty_output();
    Ok(decode_block(blk)?.oracle)
}

#[substreams::handlers::map]
pub fn map_adapter_events(blk: &eth::BlockLazyView<'_>) -> Result<proto::AdapterEvents, Error> {
    substreams::skip_empty_output();
    Ok(decode_block(blk)?.adapter)
}

#[substreams::handlers::map]
pub fn map_module_events(blk: &eth::BlockLazyView<'_>) -> Result<proto::ModuleEvents, Error> {
    substreams::skip_empty_output();
    Ok(decode_block(blk)?.modules)
}

#[substreams::handlers::map]
pub fn map_dispute_alerts(blk: &eth::BlockLazyView<'_>) -> Result<proto::DisputeAlerts, Error> {
    substreams::skip_empty_output();
    let d = decode_block(blk)?;
    let mut all_alerts = d.oracle_alerts;
    all_alerts.extend(d.adapter_alerts);

    Ok(proto::DisputeAlerts { alerts: all_alerts })
}

#[substreams::handlers::map]
pub fn map_resolution_events(
    blk: &eth::BlockLazyView<'_>,
) -> Result<proto::ResolutionEvents, Error> {
    substreams::skip_empty_output();
    let d = decode_block(blk)?;
    let mut all_alerts = d.oracle_alerts;
    all_alerts.extend(d.adapter_alerts);

    Ok(proto::ResolutionEvents {
        oracle: if has_oracle_events(&d.oracle) {
            d.oracle.into()
        } else {
            Default::default()
        },
        adapter: if has_adapter_events(&d.adapter) {
            d.adapter.into()
        } else {
            Default::default()
        },
        dispute_alerts: if !all_alerts.is_empty() {
            proto::DisputeAlerts { alerts: all_alerts }.into()
        } else {
            Default::default()
        },
        modules: if !d.modules.condition_resolved.is_empty() {
            d.modules.into()
        } else {
            Default::default()
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use substreams_ethereum::pb::eth::v2 as eth;

    #[test]
    fn test_v2_address_match() {
        let log = eth::Log {
            address: hex_literal::hex!("ee3afe347d5c74317041e2618c49534daf887c24").to_vec(),
            ..Default::default()
        };
        assert_eq!(log.address, UMA_ORACLE_V2_ADDRESS.to_vec());
    }

    #[test]
    fn test_v3_address_match() {
        let log = eth::Log {
            address: hex_literal::hex!("5953f2538f613e05baed8a5aefa8e6622467ad3d").to_vec(),
            ..Default::default()
        };
        assert_eq!(log.address, UMA_ORACLE_V3_ADDRESS.to_vec());
    }

    #[test]
    fn test_ctf_adapter_v2_address_match() {
        let log = eth::Log {
            address: hex_literal::hex!("6a9d222616c90fca5754cd1333cfd9b7fb6a4f74").to_vec(),
            ..Default::default()
        };
        assert_eq!(log.address, CTF_ADAPTER_V2_ADDRESS.to_vec());
    }

    #[test]
    fn test_ctf_adapter_v3_address_match() {
        let log = eth::Log {
            address: hex_literal::hex!("2f5e3684cb1f318ec51b00edba38d79ac2c0aa9d").to_vec(),
            ..Default::default()
        };
        assert_eq!(log.address, CTF_ADAPTER_V3_ADDRESS.to_vec());
    }

    #[test]
    fn test_is_adapter_address() {
        assert!(is_adapter_address(&CTF_ADAPTER_V2_ADDRESS));
        assert!(is_adapter_address(&CTF_ADAPTER_V3_ADDRESS));
        assert!(!is_adapter_address(&UMA_ORACLE_V2_ADDRESS));
        assert!(!is_adapter_address(&UMA_ORACLE_V3_ADDRESS));
    }

    #[test]
    fn test_all_addresses_are_20_bytes() {
        assert_eq!(UMA_ORACLE_V2_ADDRESS.len(), 20);
        assert_eq!(UMA_ORACLE_V3_ADDRESS.len(), 20);
        assert_eq!(CTF_ADAPTER_V2_ADDRESS.len(), 20);
        assert_eq!(CTF_ADAPTER_V3_ADDRESS.len(), 20);
    }

    #[test]
    fn test_all_addresses_are_distinct() {
        let addrs: Vec<&[u8; 20]> = vec![
            &UMA_ORACLE_V2_ADDRESS,
            &UMA_ORACLE_V3_ADDRESS,
            &CTF_ADAPTER_V2_ADDRESS,
            &CTF_ADAPTER_V3_ADDRESS,
        ];
        for i in 0..addrs.len() {
            for j in (i + 1)..addrs.len() {
                assert_ne!(
                    addrs[i], addrs[j],
                    "addresses at index {} and {} collide",
                    i, j
                );
            }
        }
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

    fn question_flagged_log() -> eth::Log {
        // keccak256("QuestionFlagged(bytes32)") — one indexed bytes32, no data.
        let topic0 =
            hex_literal::hex!("2435a0347185933b12027c6f394a5fd9c03646dba233e956f50658719dfc0b35")
                .to_vec();
        let question_id = [0x77u8; 32];
        eth::Log {
            address: CTF_ADAPTER_V2_ADDRESS.to_vec(),
            topics: vec![topic0, question_id.to_vec()],
            data: Vec::new(),
            ..Default::default()
        }
    }

    #[test]
    fn map_resolution_events_classifies_question_flagged() {
        let out = polymarket_substreams_common::with_lazy_block(
            &block_with_log(question_flagged_log()),
            |blk| substreams::testing::map!(map_resolution_events(blk)),
        )
        .expect("fixture block encodes")
        .expect("handler must not err");
        // A QuestionFlagged from a CTF adapter lands in adapter events, not oracle.
        assert!(!out.oracle.is_set());
        let adapter = out
            .adapter
            .as_option()
            .expect("QuestionFlagged must land in adapter events");
        assert_eq!(adapter.question_flagged.len(), 1);
        assert_eq!(
            adapter.question_flagged[0].question_id,
            [0x77u8; 32].to_vec()
        );
        // A flag must also emit exactly one dispute alert, sourced to adapter_v2.
        let alerts = out
            .dispute_alerts
            .as_option()
            .expect("QuestionFlagged must emit a dispute alert");
        assert_eq!(alerts.alerts.len(), 1);
        assert_eq!(alerts.alerts[0].alert_type, "flag");
        assert_eq!(alerts.alerts[0].source, "adapter_v2");
    }

    /// ConditionResolved(bytes31 indexed conditionId, uint256[] result) from `module`.
    fn condition_resolved_log(module: [u8; 20], payouts: &[u64]) -> eth::Log {
        let topic0 =
            hex_literal::hex!("e560c1a2847d4cf2e66de11fe592605556989449f38cc3ab96e2e287daa68caa")
                .to_vec();
        // bytes31 is left-aligned in its 32-byte topic.
        let mut condition = [0u8; 32];
        condition[..31].copy_from_slice(&[0x42u8; 31]);
        let word = |v: u64| {
            let mut w = [0u8; 32];
            w[24..].copy_from_slice(&v.to_be_bytes());
            w
        };
        let mut data = Vec::new();
        data.extend_from_slice(&word(32)); // offset of the dynamic array
        data.extend_from_slice(&word(payouts.len() as u64));
        for p in payouts {
            data.extend_from_slice(&word(*p));
        }
        eth::Log {
            address: module.to_vec(),
            topics: vec![topic0, condition.to_vec()],
            data,
            ..Default::default()
        }
    }

    #[test]
    fn map_resolution_events_decodes_module_condition_resolved() {
        for (module, name) in [
            (BINARY_MODULE_ADDRESS, "binary"),
            (NEG_RISK_MODULE_ADDRESS, "neg_risk"),
        ] {
            let block = block_with_log(condition_resolved_log(module, &[0, 1]));
            let out = polymarket_substreams_common::with_lazy_block(&block, |blk| {
                substreams::testing::map!(map_resolution_events(blk))
            })
            .expect("fixture block encodes")
            .expect("handler must not err");
            let modules = out.modules.as_option().expect("module events");
            assert_eq!(modules.condition_resolved.len(), 1);
            let ev = &modules.condition_resolved[0];
            assert_eq!(ev.condition_id, format!("0x{}", "42".repeat(31)));
            assert_eq!(ev.payouts, ["0", "1"]);
            assert_eq!(ev.module, name);
            // Module resolutions are not disputes and touch no other bucket.
            assert!(!out.oracle.is_set());
            assert!(!out.adapter.is_set());
            assert!(!out.dispute_alerts.is_set());

            let per_category = polymarket_substreams_common::with_lazy_block(&block, |blk| {
                substreams::testing::map!(map_module_events(blk))
            })
            .expect("fixture block encodes")
            .expect("handler must not err");
            assert_eq!(&per_category, modules);
        }
    }

    #[test]
    fn test_module_addresses() {
        assert!(is_module_address(&hex_literal::hex!(
            "1000008dD9001B968442c1000017eaE6E0dA00Ba"
        )));
        assert!(is_module_address(&hex_literal::hex!(
            "200000900045e3B6259600682756002200028933"
        )));
        assert!(!is_module_address(&CTF_ADAPTER_V3_ADDRESS));
        assert!(is_resolution_address(&BINARY_MODULE_ADDRESS));
    }

    #[test]
    fn map_resolution_events_empty_block_ok() {
        let out = polymarket_substreams_common::with_lazy_block(&eth::Block::default(), |blk| {
            substreams::testing::map!(map_resolution_events(blk))
        })
        .expect("fixture block encodes")
        .expect("empty block must not panic");
        assert!(!out.oracle.is_set());
        assert!(!out.adapter.is_set());
        assert!(!out.dispute_alerts.is_set());
        assert!(!out.modules.is_set());
    }
}

#[cfg(test)]
mod fixture_tests {
    use super::*;

    /// Real Polygon block 96591795 (captured with `scripts/capture_fixture.sh 96591795`). The
    /// expected counts match `substreams run` of `map_resolution_events` on the same block.
    #[test]
    fn map_resolution_events_real_block_96591795() {
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../testdata/polygon_96591795.binpb.b64"
        );
        let block = polymarket_substreams_common::load_block_b64(path).expect("fixture loads");
        assert_eq!(block.number, 96591795);
        let out = polymarket_substreams_common::with_lazy_block(&block, |blk| {
            substreams::testing::map!(map_resolution_events(blk))
        })
        .expect("fixture block encodes")
        .expect("handler must not err");
        assert_eq!(
            out.oracle.as_option().map_or(0, |e| e.settlements.len()),
            1,
            "oracle.settlements"
        );
        assert_eq!(
            out.adapter
                .as_option()
                .map_or(0, |e| e.question_resolved.len()),
            1,
            "adapter.question_resolved"
        );
        assert_eq!(
            out.adapter
                .as_option()
                .expect("adapter events")
                .question_resolved[0]
                .question_id,
            hex_literal::hex!("9d13800f3a3ac1a7e5224f60d3b1343aec81f8d66d6d7c8ffac80f9b2b331740")
        );
        assert!(!out.modules.is_set(), "no Protocol V2 module events here");
    }

    /// Real Polygon block 96681462: ConditionResolved from both Protocol V2 modules
    /// (30 BinaryModule, 11 NegRiskModule) and nothing from UMA. Values match
    /// `substreams run` of `map_resolution_events` on the same block.
    #[test]
    fn map_resolution_events_real_block_96681462_modules() {
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../testdata/polygon_96681462.binpb.b64"
        );
        let block = polymarket_substreams_common::load_block_b64(path).expect("fixture loads");
        assert_eq!(block.number, 96681462);
        let out = polymarket_substreams_common::with_lazy_block(&block, |blk| {
            substreams::testing::map!(map_resolution_events(blk))
        })
        .expect("fixture block encodes")
        .expect("handler must not err");
        assert!(!out.oracle.is_set());
        assert!(!out.adapter.is_set());
        let resolved = &out.modules.as_option().expect("module events").condition_resolved;
        let count = |m: &str| resolved.iter().filter(|e| e.module == m).count();
        assert_eq!((count("binary"), count("neg_risk")), (30, 11));

        let first = &resolved[0];
        assert_eq!(first.module, "binary");
        assert_eq!(
            first.condition_id,
            "0x01c4dffb9bd6905f7a54e9b9ad362dcbcb0000000000000000000000000000"
        );
        assert_eq!(first.payouts, ["0", "1000000"]);
        let tx = first.tx.as_option().expect("tx");
        assert_eq!(tx.log_index, 918);

        let neg_risk = resolved
            .iter()
            .find(|e| e.module == "neg_risk")
            .expect("a NegRiskModule resolution");
        assert_eq!(
            neg_risk.condition_id,
            "0x0233f39b9a277246f5ff8cb3ab3edd92000025000000000000000000000012"
        );
        assert_eq!(neg_risk.payouts, ["0", "1000000"]);

        let per_category = polymarket_substreams_common::with_lazy_block(&block, |blk| {
            substreams::testing::map!(map_module_events(blk))
        })
        .expect("fixture block encodes")
        .expect("handler must not err");
        assert_eq!(&per_category.condition_resolved, resolved);
    }
}
