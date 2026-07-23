pub mod abi;
#[allow(dead_code, clippy::all)]
pub mod pb;

use substreams::errors::Error;
use substreams_ethereum::pb::eth::v2 as eth;

use pb::polymarket::resolution::v1 as proto;
use polymarket_substreams_common::{bigint_to_string, build_tx_context, format_address};

const UMA_ORACLE_V2_ADDRESS: [u8; 20] =
    hex_literal::hex!("ee3afe347d5c74317041e2618c49534daf887c24");

const UMA_ORACLE_V3_ADDRESS: [u8; 20] =
    hex_literal::hex!("5953f2538f613e05baed8a5aefa8e6622467ad3d");

const CTF_ADAPTER_V2_ADDRESS: [u8; 20] =
    hex_literal::hex!("6a9d222616c90fca5754cd1333cfd9b7fb6a4f74");

const CTF_ADAPTER_V3_ADDRESS: [u8; 20] =
    hex_literal::hex!("2f5e3684cb1f318ec51b00edba38d79ac2c0aa9d");

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

fn to_proto_tx(
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
}

/// Single pass over `blk.logs()`, classifying each log by its emitting contract
/// via an if/else-if chain (matching the `map_all_events` convention in the other
/// packages). The transaction context is built once per event and shared between
/// the event struct and its dispute alert via `tx.clone()`.
fn decode_block(blk: &eth::Block) -> DecodedBlock {
    use abi::optimistic_oracle_v2::events as v2;
    use abi::optimistic_oracle_v3::events as v3;
    use abi::uma_ctf_adapter::events::*;

    let mut d = DecodedBlock::default();

    for log in blk.logs() {
        if log.log.address == UMA_ORACLE_V2_ADDRESS {
            if v2::ProposePrice::match_log(log.log) {
                if let Ok(e) = v2::ProposePrice::decode(log.log) {
                    let tx = to_proto_tx(blk, &log);
                    d.oracle.proposals.push(proto::ResolutionProposal {
                        oracle_version: "v2".into(),
                        proposer: format_address(&e.proposer),
                        identifier: e.identifier.to_vec(),
                        question_data: e.ancillary_data,
                        proposed_price: bigint_to_string(&e.proposed_price),
                        currency: format_address(&e.currency),
                        expiration: e.expiration_timestamp.to_u64(),
                        requester: format_address(&e.requester),
                        assertion_id: Vec::new(),
                        bond: String::new(),
                        tx: Some(tx),
                    });
                }
            } else if v2::DisputePrice::match_log(log.log) {
                if let Ok(e) = v2::DisputePrice::decode(log.log) {
                    let tx = to_proto_tx(blk, &log);
                    d.oracle.disputes.push(proto::ResolutionDispute {
                        oracle_version: "v2".into(),
                        disputer: format_address(&e.disputer),
                        identifier: e.identifier.to_vec(),
                        assertion_id: Vec::new(),
                        question_data: e.ancillary_data.clone(),
                        proposer: format_address(&e.proposer),
                        proposed_price: bigint_to_string(&e.proposed_price),
                        tx: Some(tx.clone()),
                    });
                    d.oracle_alerts.push(proto::DisputeAlert {
                        source: "oracle_v2".into(),
                        alert_type: "dispute".into(),
                        disputer: format_address(&e.disputer),
                        identifier: e.identifier.to_vec(),
                        ancillary_data: e.ancillary_data,
                        payouts: Vec::new(),
                        tx: Some(tx),
                    });
                }
            } else if v2::Settle::match_log(log.log) {
                if let Ok(e) = v2::Settle::decode(log.log) {
                    let tx = to_proto_tx(blk, &log);
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
                        tx: Some(tx),
                    });
                }
            }
        } else if log.log.address == UMA_ORACLE_V3_ADDRESS {
            if v3::AssertionMade::match_log(log.log) {
                if let Ok(e) = v3::AssertionMade::decode(log.log) {
                    let tx = to_proto_tx(blk, &log);
                    d.oracle.proposals.push(proto::ResolutionProposal {
                        oracle_version: "v3".into(),
                        proposer: format_address(&e.asserter),
                        identifier: e.identifier.to_vec(),
                        question_data: e.claim,
                        proposed_price: String::new(),
                        currency: format_address(&e.currency),
                        expiration: e.expiration_time.to_u64(),
                        requester: String::new(),
                        assertion_id: e.assertion_id.to_vec(),
                        bond: bigint_to_string(&e.bond),
                        tx: Some(tx),
                    });
                }
            } else if v3::AssertionDisputed::match_log(log.log) {
                if let Ok(e) = v3::AssertionDisputed::decode(log.log) {
                    let tx = to_proto_tx(blk, &log);
                    d.oracle.disputes.push(proto::ResolutionDispute {
                        oracle_version: "v3".into(),
                        disputer: format_address(&e.disputer),
                        identifier: Vec::new(),
                        assertion_id: e.assertion_id.to_vec(),
                        question_data: Vec::new(),
                        proposer: String::new(),
                        proposed_price: String::new(),
                        tx: Some(tx.clone()),
                    });
                    d.oracle_alerts.push(proto::DisputeAlert {
                        source: "oracle_v3".into(),
                        alert_type: "dispute".into(),
                        disputer: format_address(&e.disputer),
                        identifier: e.assertion_id.to_vec(),
                        ancillary_data: Vec::new(),
                        payouts: Vec::new(),
                        tx: Some(tx),
                    });
                }
            } else if v3::AssertionSettled::match_log(log.log) {
                if let Ok(e) = v3::AssertionSettled::decode(log.log) {
                    let tx = to_proto_tx(blk, &log);
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
                        tx: Some(tx),
                    });
                }
            }
        } else if is_adapter_address(&log.log.address) {
            let source = adapter_source(&log.log.address);

            if QuestionInitialized::match_log(log.log) {
                if let Ok(e) = QuestionInitialized::decode(log.log) {
                    let tx = to_proto_tx(blk, &log);
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
                            tx: Some(tx),
                        });
                }
            } else if QuestionResolved::match_log(log.log) {
                if let Ok(e) = QuestionResolved::decode(log.log) {
                    let tx = to_proto_tx(blk, &log);
                    d.adapter.question_resolved.push(proto::QuestionResolved {
                        question_id: e.question_id.to_vec(),
                        settled_price: bigint_to_string(&e.settled_price),
                        payouts: e.payouts.iter().map(bigint_to_string).collect(),
                        tx: Some(tx),
                    });
                }
            } else if QuestionEmergencyResolved::match_log(log.log) {
                if let Ok(e) = QuestionEmergencyResolved::decode(log.log) {
                    let tx = to_proto_tx(blk, &log);
                    let payouts: Vec<String> = e.payouts.iter().map(bigint_to_string).collect();
                    d.adapter
                        .question_emergency_resolved
                        .push(proto::QuestionEmergencyResolved {
                            question_id: e.question_id.to_vec(),
                            payouts: payouts.clone(),
                            tx: Some(tx.clone()),
                        });
                    d.adapter_alerts.push(proto::DisputeAlert {
                        source: source.into(),
                        alert_type: "emergency_resolve".into(),
                        disputer: String::new(),
                        identifier: e.question_id.to_vec(),
                        ancillary_data: Vec::new(),
                        payouts,
                        tx: Some(tx),
                    });
                }
            } else if QuestionFlagged::match_log(log.log) {
                if let Ok(e) = QuestionFlagged::decode(log.log) {
                    let tx = to_proto_tx(blk, &log);
                    d.adapter.question_flagged.push(proto::QuestionFlagged {
                        question_id: e.question_id.to_vec(),
                        tx: Some(tx.clone()),
                    });
                    d.adapter_alerts.push(proto::DisputeAlert {
                        source: source.into(),
                        alert_type: "flag".into(),
                        disputer: String::new(),
                        identifier: e.question_id.to_vec(),
                        ancillary_data: Vec::new(),
                        payouts: Vec::new(),
                        tx: Some(tx),
                    });
                }
            } else if QuestionUnflagged::match_log(log.log) {
                if let Ok(e) = QuestionUnflagged::decode(log.log) {
                    let tx = to_proto_tx(blk, &log);
                    d.adapter.question_unflagged.push(proto::QuestionUnflagged {
                        question_id: e.question_id.to_vec(),
                        tx: Some(tx),
                    });
                }
            } else if QuestionPaused::match_log(log.log) {
                if let Ok(e) = QuestionPaused::decode(log.log) {
                    let tx = to_proto_tx(blk, &log);
                    d.adapter.question_paused.push(proto::QuestionPaused {
                        question_id: e.question_id.to_vec(),
                        tx: Some(tx.clone()),
                    });
                    d.adapter_alerts.push(proto::DisputeAlert {
                        source: source.into(),
                        alert_type: "pause".into(),
                        disputer: String::new(),
                        identifier: e.question_id.to_vec(),
                        ancillary_data: Vec::new(),
                        payouts: Vec::new(),
                        tx: Some(tx),
                    });
                }
            } else if QuestionUnpaused::match_log(log.log) {
                if let Ok(e) = QuestionUnpaused::decode(log.log) {
                    let tx = to_proto_tx(blk, &log);
                    d.adapter.question_unpaused.push(proto::QuestionUnpaused {
                        question_id: e.question_id.to_vec(),
                        tx: Some(tx),
                    });
                }
            } else if QuestionReset::match_log(log.log) {
                if let Ok(e) = QuestionReset::decode(log.log) {
                    let tx = to_proto_tx(blk, &log);
                    d.adapter.question_reset.push(proto::QuestionReset {
                        question_id: e.question_id.to_vec(),
                        tx: Some(tx.clone()),
                    });
                    d.adapter_alerts.push(proto::DisputeAlert {
                        source: source.into(),
                        alert_type: "reset".into(),
                        disputer: String::new(),
                        identifier: e.question_id.to_vec(),
                        ancillary_data: Vec::new(),
                        payouts: Vec::new(),
                        tx: Some(tx),
                    });
                }
            } else if AncillaryDataUpdated::match_log(log.log) {
                if let Ok(e) = AncillaryDataUpdated::decode(log.log) {
                    let tx = to_proto_tx(blk, &log);
                    d.adapter
                        .ancillary_data_updated
                        .push(proto::AncillaryDataUpdated {
                            question_id: e.question_id.to_vec(),
                            owner: format_address(&e.owner),
                            update_data: e.update,
                            tx: Some(tx),
                        });
                }
            } else if NewAdmin::match_log(log.log) {
                if let Ok(e) = NewAdmin::decode(log.log) {
                    let tx = to_proto_tx(blk, &log);
                    d.adapter.admin_changes.push(proto::AdapterAdmin {
                        admin: format_address(&e.admin),
                        target_admin: format_address(&e.new_admin_address),
                        is_addition: true,
                        tx: Some(tx),
                    });
                }
            } else if RemovedAdmin::match_log(log.log) {
                if let Ok(e) = RemovedAdmin::decode(log.log) {
                    let tx = to_proto_tx(blk, &log);
                    d.adapter.admin_changes.push(proto::AdapterAdmin {
                        admin: format_address(&e.admin),
                        target_admin: format_address(&e.removed_admin),
                        is_addition: false,
                        tx: Some(tx),
                    });
                }
            }
        }
    }

    d
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
pub fn map_oracle_events(blk: eth::Block) -> Result<proto::OracleEvents, Error> {
    Ok(decode_block(&blk).oracle)
}

#[substreams::handlers::map]
pub fn map_adapter_events(blk: eth::Block) -> Result<proto::AdapterEvents, Error> {
    Ok(decode_block(&blk).adapter)
}

#[substreams::handlers::map]
pub fn map_dispute_alerts(blk: eth::Block) -> Result<proto::DisputeAlerts, Error> {
    let d = decode_block(&blk);
    let mut all_alerts = d.oracle_alerts;
    all_alerts.extend(d.adapter_alerts);

    Ok(proto::DisputeAlerts { alerts: all_alerts })
}

#[substreams::handlers::map]
pub fn map_resolution_events(blk: eth::Block) -> Result<proto::ResolutionEvents, Error> {
    let d = decode_block(&blk);
    let mut all_alerts = d.oracle_alerts;
    all_alerts.extend(d.adapter_alerts);

    Ok(proto::ResolutionEvents {
        oracle: if has_oracle_events(&d.oracle) {
            Some(d.oracle)
        } else {
            None
        },
        adapter: if has_adapter_events(&d.adapter) {
            Some(d.adapter)
        } else {
            None
        },
        dispute_alerts: if !all_alerts.is_empty() {
            Some(proto::DisputeAlerts { alerts: all_alerts })
        } else {
            None
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
