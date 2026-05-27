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

fn to_proto_tx(blk: &eth::Block, log: &substreams_ethereum::block_view::LogView) -> proto::TransactionContext {
    let ctx = build_tx_context(blk, log);
    proto::TransactionContext {
        tx_hash: ctx.tx_hash,
        log_index: ctx.log_index,
        block_number: ctx.block_number,
        timestamp: ctx.timestamp,
    }
}

#[substreams::handlers::map]
pub fn map_v2_events(blk: eth::Block) -> Result<proto::OracleV2Events, Error> {
    use abi::optimistic_oracle_v2::events::*;

    let mut events = proto::OracleV2Events::default();

    for log in blk.logs() {
        if log.log.address != UMA_ORACLE_V2_ADDRESS {
            continue;
        }

        if ProposePrice::match_log(log.log) {
            if let Ok(event) = ProposePrice::decode(log.log) {
                events.propose_price.push(proto::ProposePrice {
                    requester: format_address(&event.requester),
                    proposer: format_address(&event.proposer),
                    identifier: event.identifier.to_vec(),
                    timestamp: bigint_to_string(&event.timestamp),
                    ancillary_data: event.ancillary_data,
                    proposed_price: bigint_to_string(&event.proposed_price),
                    expiration_timestamp: bigint_to_string(&event.expiration_timestamp),
                    currency: format_address(&event.currency),
                    tx: Some(to_proto_tx(&blk, &log)),
                });
            }
        } else if DisputePrice::match_log(log.log) {
            if let Ok(event) = DisputePrice::decode(log.log) {
                events.dispute_price.push(proto::DisputePrice {
                    requester: format_address(&event.requester),
                    proposer: format_address(&event.proposer),
                    disputer: format_address(&event.disputer),
                    identifier: event.identifier.to_vec(),
                    timestamp: bigint_to_string(&event.timestamp),
                    ancillary_data: event.ancillary_data,
                    proposed_price: bigint_to_string(&event.proposed_price),
                    tx: Some(to_proto_tx(&blk, &log)),
                });
            }
        } else if Settle::match_log(log.log) {
            if let Ok(event) = Settle::decode(log.log) {
                events.settle.push(proto::SettleV2 {
                    requester: format_address(&event.requester),
                    proposer: format_address(&event.proposer),
                    disputer: format_address(&event.disputer),
                    identifier: event.identifier.to_vec(),
                    timestamp: bigint_to_string(&event.timestamp),
                    ancillary_data: event.ancillary_data,
                    price: bigint_to_string(&event.price),
                    payout: bigint_to_string(&event.payout),
                    tx: Some(to_proto_tx(&blk, &log)),
                });
            }
        }
    }

    Ok(events)
}

#[substreams::handlers::map]
pub fn map_v3_events(blk: eth::Block) -> Result<proto::OracleV3Events, Error> {
    use abi::optimistic_oracle_v3::events::*;

    let mut events = proto::OracleV3Events::default();

    for log in blk.logs() {
        if log.log.address != UMA_ORACLE_V3_ADDRESS {
            continue;
        }

        if AssertionMade::match_log(log.log) {
            if let Ok(event) = AssertionMade::decode(log.log) {
                events.assertion_made.push(proto::AssertionMade {
                    assertion_id: event.assertion_id.to_vec(),
                    domain_id: event.domain_id.to_vec(),
                    claim: event.claim,
                    asserter: format_address(&event.asserter),
                    callback_recipient: format_address(&event.callback_recipient),
                    escalation_manager: format_address(&event.escalation_manager),
                    caller: format_address(&event.caller),
                    expiration_time: event.expiration_time.to_u64(),
                    currency: format_address(&event.currency),
                    bond: bigint_to_string(&event.bond),
                    identifier: event.identifier.to_vec(),
                    tx: Some(to_proto_tx(&blk, &log)),
                });
            }
        } else if AssertionDisputed::match_log(log.log) {
            if let Ok(event) = AssertionDisputed::decode(log.log) {
                events.assertion_disputed.push(proto::AssertionDisputed {
                    assertion_id: event.assertion_id.to_vec(),
                    caller: format_address(&event.caller),
                    disputer: format_address(&event.disputer),
                    tx: Some(to_proto_tx(&blk, &log)),
                });
            }
        } else if AssertionSettled::match_log(log.log) {
            if let Ok(event) = AssertionSettled::decode(log.log) {
                events.assertion_settled.push(proto::AssertionSettled {
                    assertion_id: event.assertion_id.to_vec(),
                    bond_recipient: format_address(&event.bond_recipient),
                    disputed: event.disputed,
                    settlement_resolution: event.settlement_resolution,
                    settle_caller: format_address(&event.settle_caller),
                    tx: Some(to_proto_tx(&blk, &log)),
                });
            }
        }
    }

    Ok(events)
}

#[substreams::handlers::map]
pub fn map_dispute_alerts(blk: eth::Block) -> Result<proto::DisputeAlerts, Error> {
    let mut alerts = proto::DisputeAlerts::default();

    for log in blk.logs() {
        if log.log.address == UMA_ORACLE_V2_ADDRESS {
            use abi::optimistic_oracle_v2::events::DisputePrice;
            if DisputePrice::match_log(log.log) {
                if let Ok(event) = DisputePrice::decode(log.log) {
                    alerts.alerts.push(proto::DisputeAlert {
                        source: "v2".to_string(),
                        alert_type: "dispute".to_string(),
                        disputer: format_address(&event.disputer),
                        identifier: event.identifier.to_vec(),
                        ancillary_data: event.ancillary_data,
                        payouts: Vec::new(),
                        tx: Some(to_proto_tx(&blk, &log)),
                    });
                }
            }
        } else if log.log.address == UMA_ORACLE_V3_ADDRESS {
            use abi::optimistic_oracle_v3::events::AssertionDisputed;
            if AssertionDisputed::match_log(log.log) {
                if let Ok(event) = AssertionDisputed::decode(log.log) {
                    alerts.alerts.push(proto::DisputeAlert {
                        source: "v3".to_string(),
                        alert_type: "dispute".to_string(),
                        disputer: format_address(&event.disputer),
                        identifier: event.assertion_id.to_vec(),
                        ancillary_data: Vec::new(),
                        payouts: Vec::new(),
                        tx: Some(to_proto_tx(&blk, &log)),
                    });
                }
            }
        }
    }

    Ok(alerts)
}

#[substreams::handlers::map]
pub fn map_all_events(blk: eth::Block) -> Result<proto::ResolutionEvents, Error> {
    use abi::optimistic_oracle_v2::events as v2_events;
    use abi::optimistic_oracle_v3::events as v3_events;

    let mut v2 = proto::OracleV2Events::default();
    let mut v3 = proto::OracleV3Events::default();
    let mut disputes = proto::DisputeAlerts::default();

    for log in blk.logs() {
        if log.log.address == UMA_ORACLE_V2_ADDRESS {
            if v2_events::ProposePrice::match_log(log.log) {
                if let Ok(event) = v2_events::ProposePrice::decode(log.log) {
                    v2.propose_price.push(proto::ProposePrice {
                        requester: format_address(&event.requester),
                        proposer: format_address(&event.proposer),
                        identifier: event.identifier.to_vec(),
                        timestamp: bigint_to_string(&event.timestamp),
                        ancillary_data: event.ancillary_data,
                        proposed_price: bigint_to_string(&event.proposed_price),
                        expiration_timestamp: bigint_to_string(&event.expiration_timestamp),
                        currency: format_address(&event.currency),
                        tx: Some(to_proto_tx(&blk, &log)),
                    });
                }
            } else if v2_events::DisputePrice::match_log(log.log) {
                if let Ok(event) = v2_events::DisputePrice::decode(log.log) {
                    let dp = proto::DisputePrice {
                        requester: format_address(&event.requester),
                        proposer: format_address(&event.proposer),
                        disputer: format_address(&event.disputer),
                        identifier: event.identifier.to_vec(),
                        timestamp: bigint_to_string(&event.timestamp),
                        ancillary_data: event.ancillary_data.clone(),
                        proposed_price: bigint_to_string(&event.proposed_price),
                        tx: Some(to_proto_tx(&blk, &log)),
                    };
                    disputes.alerts.push(proto::DisputeAlert {
                        source: "v2".to_string(),
                        alert_type: "dispute".to_string(),
                        disputer: format_address(&event.disputer),
                        identifier: event.identifier.to_vec(),
                        ancillary_data: event.ancillary_data,
                        payouts: Vec::new(),
                        tx: Some(to_proto_tx(&blk, &log)),
                    });
                    v2.dispute_price.push(dp);
                }
            } else if v2_events::Settle::match_log(log.log) {
                if let Ok(event) = v2_events::Settle::decode(log.log) {
                    v2.settle.push(proto::SettleV2 {
                        requester: format_address(&event.requester),
                        proposer: format_address(&event.proposer),
                        disputer: format_address(&event.disputer),
                        identifier: event.identifier.to_vec(),
                        timestamp: bigint_to_string(&event.timestamp),
                        ancillary_data: event.ancillary_data,
                        price: bigint_to_string(&event.price),
                        payout: bigint_to_string(&event.payout),
                        tx: Some(to_proto_tx(&blk, &log)),
                    });
                }
            }
        } else if log.log.address == UMA_ORACLE_V3_ADDRESS {
            if v3_events::AssertionMade::match_log(log.log) {
                if let Ok(event) = v3_events::AssertionMade::decode(log.log) {
                    v3.assertion_made.push(proto::AssertionMade {
                        assertion_id: event.assertion_id.to_vec(),
                        domain_id: event.domain_id.to_vec(),
                        claim: event.claim,
                        asserter: format_address(&event.asserter),
                        callback_recipient: format_address(&event.callback_recipient),
                        escalation_manager: format_address(&event.escalation_manager),
                        caller: format_address(&event.caller),
                        expiration_time: event.expiration_time.to_u64(),
                        currency: format_address(&event.currency),
                        bond: bigint_to_string(&event.bond),
                        identifier: event.identifier.to_vec(),
                        tx: Some(to_proto_tx(&blk, &log)),
                    });
                }
            } else if v3_events::AssertionDisputed::match_log(log.log) {
                if let Ok(event) = v3_events::AssertionDisputed::decode(log.log) {
                    disputes.alerts.push(proto::DisputeAlert {
                        source: "v3".to_string(),
                        alert_type: "dispute".to_string(),
                        disputer: format_address(&event.disputer),
                        identifier: event.assertion_id.to_vec(),
                        ancillary_data: Vec::new(),
                        payouts: Vec::new(),
                        tx: Some(to_proto_tx(&blk, &log)),
                    });
                    v3.assertion_disputed.push(proto::AssertionDisputed {
                        assertion_id: event.assertion_id.to_vec(),
                        caller: format_address(&event.caller),
                        disputer: format_address(&event.disputer),
                        tx: Some(to_proto_tx(&blk, &log)),
                    });
                }
            } else if v3_events::AssertionSettled::match_log(log.log) {
                if let Ok(event) = v3_events::AssertionSettled::decode(log.log) {
                    v3.assertion_settled.push(proto::AssertionSettled {
                        assertion_id: event.assertion_id.to_vec(),
                        bond_recipient: format_address(&event.bond_recipient),
                        disputed: event.disputed,
                        settlement_resolution: event.settlement_resolution,
                        settle_caller: format_address(&event.settle_caller),
                        tx: Some(to_proto_tx(&blk, &log)),
                    });
                }
            }
        }
    }

    let has_v2 = !v2.propose_price.is_empty()
        || !v2.dispute_price.is_empty()
        || !v2.settle.is_empty();
    let has_v3 = !v3.assertion_made.is_empty()
        || !v3.assertion_disputed.is_empty()
        || !v3.assertion_settled.is_empty();

    Ok(proto::ResolutionEvents {
        oracle_v2: if has_v2 { Some(v2) } else { None },
        oracle_v3: if has_v3 { Some(v3) } else { None },
        adapter: None,
        dispute_alerts: if !disputes.alerts.is_empty() { Some(disputes) } else { None },
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
    fn test_addresses_are_20_bytes() {
        assert_eq!(UMA_ORACLE_V2_ADDRESS.len(), 20);
        assert_eq!(UMA_ORACLE_V3_ADDRESS.len(), 20);
    }

    #[test]
    fn test_v2_and_v3_are_different() {
        assert_ne!(UMA_ORACLE_V2_ADDRESS, UMA_ORACLE_V3_ADDRESS);
    }
}
