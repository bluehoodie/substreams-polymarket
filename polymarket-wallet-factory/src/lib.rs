#[allow(dead_code, clippy::all)]
pub mod pb;
pub mod abi;

use substreams::errors::Error;
use substreams_ethereum::pb::eth::v2 as eth;

use pb::polymarket::wallet_factory::v1 as proto;
use pb::sf::substreams::index::v1::Keys;
use polymarket_substreams_common::{build_tx_context, format_address};

const DEPOSIT_WALLET_FACTORY_ADDRESS: [u8; 20] = hex_literal::hex!("00000000000Fb5C9ADea0298D729A0CB3823Cc07");

/// Block index module: emits one `evt_addr:<address>` key per block containing a
/// Deposit Wallet Factory contract log, so downstream modules can skip blocks that
/// never touch it (the data is sparse — a single contract on Polygon).
#[substreams::handlers::map]
pub fn index_events(blk: eth::Block) -> Result<Keys, Error> {
    let mut keys = Keys::default();
    for log in blk.logs() {
        if let Some(key) = index_key_for_address(&log.log.address) {
            if !keys.keys.contains(&key) {
                keys.keys.push(key);
            }
        }
    }
    Ok(keys)
}

/// Returns the block-index key for a log address, or `None` if it is not a
/// contract this package targets. The returned string must match the
/// `blockFilter` query in `substreams.yaml` exactly (lowercase hex, `0x` prefix).
fn index_key_for_address(addr: &[u8]) -> Option<String> {
    if addr == DEPOSIT_WALLET_FACTORY_ADDRESS {
        Some(format!("evt_addr:{}", format_address(addr)))
    } else {
        None
    }
}

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
                events.implementation_authorized.push(proto::ImplementationAuthorized {
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
fn build_transaction_context(blk: &eth::Block, log: &substreams_ethereum::block_view::LogView) -> proto::TransactionContext {
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
mod index_tests {
    use super::*;

    #[test]
    fn test_index_key_for_factory_address() {
        let addr = hex_literal::hex!("00000000000Fb5C9ADea0298D729A0CB3823Cc07");
        assert_eq!(
            index_key_for_address(&addr),
            Some("evt_addr:0x00000000000fb5c9adea0298d729a0cb3823cc07".to_string())
        );
    }

    #[test]
    fn test_index_key_for_unrelated_address() {
        let addr = hex_literal::hex!("00000000000000000000000000000000000000ff");
        assert_eq!(index_key_for_address(&addr), None);
    }
}
