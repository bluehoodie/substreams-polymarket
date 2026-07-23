pub mod abi;
#[allow(dead_code, clippy::all)]
pub mod pb;

use substreams::errors::Error;
use substreams_ethereum::pb::eth::v2 as eth;

use pb::polymarket::wallet_factory::v1 as proto;
use polymarket_substreams_common::{
    build_tx_context, format_address, DEPOSIT_WALLET_FACTORY as DEPOSIT_WALLET_FACTORY_ADDRESS,
};

#[substreams::handlers::map]
pub fn map_factory_events(blk: eth::Block) -> Result<proto::FactoryEvents, Error> {
    use abi::deposit_wallet_factory::events::*;

    let mut events = proto::FactoryEvents::default();

    for log in blk.logs() {
        if !is_factory_contract(log.log) {
            continue;
        }

        if WalletDeployed::match_log(log.log) {
            if let Ok(event) = WalletDeployed::decode(log.log) {
                events.wallet_deployed.push(proto::WalletDeployed {
                    wallet: format_address(&event.wallet),
                    owner: format_address(&event.owner),
                    id: format!("0x{}", hex::encode(event.id)),
                    implementation: format_address(&event.implementation),
                    tx: Some(build_transaction_context(&blk, &log)),
                });
            }
        } else if ImplementationAuthorized::match_log(log.log) {
            if let Ok(event) = ImplementationAuthorized::decode(log.log) {
                events
                    .implementation_authorized
                    .push(proto::ImplementationAuthorized {
                        implementation: format_address(&event.implementation),
                        tx: Some(build_transaction_context(&blk, &log)),
                    });
            }
        }
    }

    Ok(events)
}

#[inline]
fn is_factory_contract(log: &eth::Log) -> bool {
    log.address == DEPOSIT_WALLET_FACTORY_ADDRESS
}

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
    use substreams_ethereum::pb::eth::v2 as eth;

    #[test]
    fn test_is_factory_contract_matching_address() {
        let log = eth::Log {
            address: hex_literal::hex!("00000000000Fb5C9ADea0298D729A0CB3823Cc07").to_vec(),
            ..Default::default()
        };
        assert!(is_factory_contract(&log));
    }

    #[test]
    fn test_is_factory_contract_non_matching_address() {
        let log = eth::Log {
            address: hex_literal::hex!("0000000000000000000000000000000000000000").to_vec(),
            ..Default::default()
        };
        assert!(!is_factory_contract(&log));
    }

    #[test]
    fn test_factory_contract_address_is_20_bytes() {
        assert_eq!(DEPOSIT_WALLET_FACTORY_ADDRESS.len(), 20);
    }

    #[test]
    fn test_format_address() {
        let bytes = hex_literal::hex!("00000000000Fb5C9ADea0298D729A0CB3823Cc07");
        let result = format_address(&bytes);
        assert_eq!(result, "0x00000000000fb5c9adea0298d729a0cb3823cc07");
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

    fn wallet_deployed_log() -> eth::Log {
        let topic0 =
            hex_literal::hex!("7441de0ad639fe5d2bf1c22447715a0528b682385736bb40ae8dd92555eb8276")
                .to_vec();
        let wallet = [0x44u8; 20];
        let owner = [0x55u8; 20];
        let id = [0x66u8; 32];
        let impl_addr = [0x99u8; 20];
        let mut impl_word = [0u8; 32];
        impl_word[12..].copy_from_slice(&impl_addr);
        eth::Log {
            address: DEPOSIT_WALLET_FACTORY_ADDRESS.to_vec(),
            topics: vec![topic0, topic_addr(&wallet), topic_addr(&owner), id.to_vec()],
            data: impl_word.to_vec(),
            ..Default::default()
        }
    }

    #[test]
    fn map_factory_events_classifies_wallet_deployed() {
        let out =
            __impl_map_factory_events(block_with_log(wallet_deployed_log())).expect("handler must not err");
        assert_eq!(out.wallet_deployed.len(), 1);
        assert_eq!(
            out.wallet_deployed[0].wallet,
            "0x4444444444444444444444444444444444444444"
        );
        assert!(out.implementation_authorized.is_empty());
    }

    #[test]
    fn map_factory_events_empty_block_ok() {
        let out = __impl_map_factory_events(eth::Block::default()).expect("empty block must not panic");
        assert!(out.wallet_deployed.is_empty());
        assert!(out.implementation_authorized.is_empty());
    }
}
