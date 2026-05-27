#[allow(dead_code, clippy::all)]
pub mod pb;
pub mod abi;

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

fn to_proto_tx(blk: &eth::Block, log: &substreams_ethereum::block_view::LogView) -> proto::TransactionContext {
    let ctx = build_tx_context(blk, log);
    proto::TransactionContext {
        tx_hash: ctx.tx_hash,
        log_index: ctx.log_index,
        block_number: ctx.block_number,
        timestamp: ctx.timestamp,
    }
}

fn decode_oracle_events(blk: &eth::Block) -> (proto::OracleEvents, Vec<proto::DisputeAlert>) {
    use abi::optimistic_oracle_v2::events as v2;
    use abi::optimistic_oracle_v3::events as v3;

    let mut oracle = proto::OracleEvents::default();
    let mut alerts = Vec::new();

    for log in blk.logs() {
        if log.log.address == UMA_ORACLE_V2_ADDRESS {
            if v2::ProposePrice::match_log(log.log) {
                if let Ok(e) = v2::ProposePrice::decode(log.log) {
                    oracle.proposals.push(proto::ResolutionProposal {
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
                        tx: Some(to_proto_tx(blk, &log)),
                    });
                }
            } else if v2::DisputePrice::match_log(log.log) {
                if let Ok(e) = v2::DisputePrice::decode(log.log) {
                    oracle.disputes.push(proto::ResolutionDispute {
                        oracle_version: "v2".into(),
                        disputer: format_address(&e.disputer),
                        identifier: e.identifier.to_vec(),
                        assertion_id: Vec::new(),
                        question_data: e.ancillary_data.clone(),
                        proposer: format_address(&e.proposer),
                        proposed_price: bigint_to_string(&e.proposed_price),
                        tx: Some(to_proto_tx(blk, &log)),
                    });
                    alerts.push(proto::DisputeAlert {
                        source: "oracle_v2".into(),
                        alert_type: "dispute".into(),
                        disputer: format_address(&e.disputer),
                        identifier: e.identifier.to_vec(),
                        ancillary_data: e.ancillary_data,
                        payouts: Vec::new(),
                        tx: Some(to_proto_tx(blk, &log)),
                    });
                }
            } else if v2::Settle::match_log(log.log) {
                if let Ok(e) = v2::Settle::decode(log.log) {
                    oracle.settlements.push(proto::ResolutionSettlement {
                        oracle_version: "v2".into(),
                        price: bigint_to_string(&e.price),
                        disputed: !e.disputer.iter().all(|&b| b == 0),
                        proposer: format_address(&e.proposer),
                        disputer: format_address(&e.disputer),
                        identifier: e.identifier.to_vec(),
                        assertion_id: Vec::new(),
                        payout: bigint_to_string(&e.payout),
                        question_data: e.ancillary_data,
                        tx: Some(to_proto_tx(blk, &log)),
                    });
                }
            }
        } else if log.log.address == UMA_ORACLE_V3_ADDRESS {
            if v3::AssertionMade::match_log(log.log) {
                if let Ok(e) = v3::AssertionMade::decode(log.log) {
                    oracle.proposals.push(proto::ResolutionProposal {
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
                        tx: Some(to_proto_tx(blk, &log)),
                    });
                }
            } else if v3::AssertionDisputed::match_log(log.log) {
                if let Ok(e) = v3::AssertionDisputed::decode(log.log) {
                    oracle.disputes.push(proto::ResolutionDispute {
                        oracle_version: "v3".into(),
                        disputer: format_address(&e.disputer),
                        identifier: Vec::new(),
                        assertion_id: e.assertion_id.to_vec(),
                        question_data: Vec::new(),
                        proposer: String::new(),
                        proposed_price: String::new(),
                        tx: Some(to_proto_tx(blk, &log)),
                    });
                    alerts.push(proto::DisputeAlert {
                        source: "oracle_v3".into(),
                        alert_type: "dispute".into(),
                        disputer: format_address(&e.disputer),
                        identifier: e.assertion_id.to_vec(),
                        ancillary_data: Vec::new(),
                        payouts: Vec::new(),
                        tx: Some(to_proto_tx(blk, &log)),
                    });
                }
            } else if v3::AssertionSettled::match_log(log.log) {
                if let Ok(e) = v3::AssertionSettled::decode(log.log) {
                    oracle.settlements.push(proto::ResolutionSettlement {
                        oracle_version: "v3".into(),
                        price: if e.settlement_resolution { "1" } else { "0" }.into(),
                        disputed: e.disputed,
                        proposer: String::new(),
                        disputer: String::new(),
                        identifier: Vec::new(),
                        assertion_id: e.assertion_id.to_vec(),
                        payout: String::new(),
                        question_data: Vec::new(),
                        tx: Some(to_proto_tx(blk, &log)),
                    });
                }
            }
        }
    }

    (oracle, alerts)
}

fn decode_adapter_events(blk: &eth::Block) -> (proto::AdapterEvents, Vec<proto::DisputeAlert>) {
    use abi::uma_ctf_adapter::events::*;

    let mut adapter = proto::AdapterEvents::default();
    let mut alerts = Vec::new();

    for log in blk.logs() {
        if !is_adapter_address(&log.log.address) {
            continue;
        }
        let source = adapter_source(&log.log.address);

        if QuestionInitialized::match_log(log.log) {
            if let Ok(e) = QuestionInitialized::decode(log.log) {
                adapter.question_initialized.push(proto::QuestionInitialized {
                    question_id: e.question_id.to_vec(),
                    request_timestamp: bigint_to_string(&e.request_timestamp),
                    creator: format_address(&e.creator),
                    ancillary_data: e.ancillary_data,
                    reward_token: format_address(&e.reward_token),
                    reward: bigint_to_string(&e.reward),
                    proposal_bond: bigint_to_string(&e.proposal_bond),
                    tx: Some(to_proto_tx(blk, &log)),
                });
            }
        } else if QuestionResolved::match_log(log.log) {
            if let Ok(e) = QuestionResolved::decode(log.log) {
                adapter.question_resolved.push(proto::QuestionResolved {
                    question_id: e.question_id.to_vec(),
                    settled_price: bigint_to_string(&e.settled_price),
                    payouts: e.payouts.iter().map(|p| bigint_to_string(p)).collect(),
                    tx: Some(to_proto_tx(blk, &log)),
                });
            }
        } else if QuestionEmergencyResolved::match_log(log.log) {
            if let Ok(e) = QuestionEmergencyResolved::decode(log.log) {
                let payouts: Vec<String> = e.payouts.iter().map(|p| bigint_to_string(p)).collect();
                adapter.question_emergency_resolved.push(proto::QuestionEmergencyResolved {
                    question_id: e.question_id.to_vec(),
                    payouts: payouts.clone(),
                    tx: Some(to_proto_tx(blk, &log)),
                });
                alerts.push(proto::DisputeAlert {
                    source: source.into(),
                    alert_type: "emergency_resolve".into(),
                    disputer: String::new(),
                    identifier: e.question_id.to_vec(),
                    ancillary_data: Vec::new(),
                    payouts,
                    tx: Some(to_proto_tx(blk, &log)),
                });
            }
        } else if QuestionFlagged::match_log(log.log) {
            if let Ok(e) = QuestionFlagged::decode(log.log) {
                adapter.question_flagged.push(proto::QuestionFlagged {
                    question_id: e.question_id.to_vec(),
                    tx: Some(to_proto_tx(blk, &log)),
                });
                alerts.push(proto::DisputeAlert {
                    source: source.into(),
                    alert_type: "flag".into(),
                    disputer: String::new(),
                    identifier: e.question_id.to_vec(),
                    ancillary_data: Vec::new(),
                    payouts: Vec::new(),
                    tx: Some(to_proto_tx(blk, &log)),
                });
            }
        } else if QuestionUnflagged::match_log(log.log) {
            if let Ok(e) = QuestionUnflagged::decode(log.log) {
                adapter.question_unflagged.push(proto::QuestionUnflagged {
                    question_id: e.question_id.to_vec(),
                    tx: Some(to_proto_tx(blk, &log)),
                });
            }
        } else if QuestionPaused::match_log(log.log) {
            if let Ok(e) = QuestionPaused::decode(log.log) {
                adapter.question_paused.push(proto::QuestionPaused {
                    question_id: e.question_id.to_vec(),
                    tx: Some(to_proto_tx(blk, &log)),
                });
                alerts.push(proto::DisputeAlert {
                    source: source.into(),
                    alert_type: "pause".into(),
                    disputer: String::new(),
                    identifier: e.question_id.to_vec(),
                    ancillary_data: Vec::new(),
                    payouts: Vec::new(),
                    tx: Some(to_proto_tx(blk, &log)),
                });
            }
        } else if QuestionUnpaused::match_log(log.log) {
            if let Ok(e) = QuestionUnpaused::decode(log.log) {
                adapter.question_unpaused.push(proto::QuestionUnpaused {
                    question_id: e.question_id.to_vec(),
                    tx: Some(to_proto_tx(blk, &log)),
                });
            }
        } else if QuestionReset::match_log(log.log) {
            if let Ok(e) = QuestionReset::decode(log.log) {
                adapter.question_reset.push(proto::QuestionReset {
                    question_id: e.question_id.to_vec(),
                    tx: Some(to_proto_tx(blk, &log)),
                });
                alerts.push(proto::DisputeAlert {
                    source: source.into(),
                    alert_type: "reset".into(),
                    disputer: String::new(),
                    identifier: e.question_id.to_vec(),
                    ancillary_data: Vec::new(),
                    payouts: Vec::new(),
                    tx: Some(to_proto_tx(blk, &log)),
                });
            }
        } else if AncillaryDataUpdated::match_log(log.log) {
            if let Ok(e) = AncillaryDataUpdated::decode(log.log) {
                adapter.ancillary_data_updated.push(proto::AncillaryDataUpdated {
                    question_id: e.question_id.to_vec(),
                    owner: format_address(&e.owner),
                    update_data: e.update,
                    tx: Some(to_proto_tx(blk, &log)),
                });
            }
        } else if NewAdmin::match_log(log.log) {
            if let Ok(e) = NewAdmin::decode(log.log) {
                adapter.admin_changes.push(proto::AdapterAdmin {
                    admin: format_address(&e.admin),
                    target_admin: format_address(&e.new_admin_address),
                    is_addition: true,
                    tx: Some(to_proto_tx(blk, &log)),
                });
            }
        } else if RemovedAdmin::match_log(log.log) {
            if let Ok(e) = RemovedAdmin::decode(log.log) {
                adapter.admin_changes.push(proto::AdapterAdmin {
                    admin: format_address(&e.admin),
                    target_admin: format_address(&e.removed_admin),
                    is_addition: false,
                    tx: Some(to_proto_tx(blk, &log)),
                });
            }
        }
    }

    (adapter, alerts)
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
    let (oracle, _) = decode_oracle_events(&blk);
    Ok(oracle)
}

#[substreams::handlers::map]
pub fn map_adapter_events(blk: eth::Block) -> Result<proto::AdapterEvents, Error> {
    let (adapter, _) = decode_adapter_events(&blk);
    Ok(adapter)
}

#[substreams::handlers::map]
pub fn map_dispute_alerts(blk: eth::Block) -> Result<proto::DisputeAlerts, Error> {
    let (_, oracle_alerts) = decode_oracle_events(&blk);
    let (_, adapter_alerts) = decode_adapter_events(&blk);

    let mut all_alerts = oracle_alerts;
    all_alerts.extend(adapter_alerts);

    Ok(proto::DisputeAlerts { alerts: all_alerts })
}

#[substreams::handlers::map]
pub fn map_resolution_events(blk: eth::Block) -> Result<proto::ResolutionEvents, Error> {
    let (oracle, oracle_alerts) = decode_oracle_events(&blk);
    let (adapter, adapter_alerts) = decode_adapter_events(&blk);

    let mut all_alerts = oracle_alerts;
    all_alerts.extend(adapter_alerts);

    Ok(proto::ResolutionEvents {
        oracle: if has_oracle_events(&oracle) { Some(oracle) } else { None },
        adapter: if has_adapter_events(&adapter) { Some(adapter) } else { None },
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
                assert_ne!(addrs[i], addrs[j], "addresses at index {} and {} collide", i, j);
            }
        }
    }
}
