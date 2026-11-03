//! The lazy block helpers must select exactly what the owned `Block` path
//! selected: same logs, same order, same transaction hash, log index, block
//! number and timestamp. Handlers only changed how they reach a log; the
//! Abigen decode they run on it is the same code, so this is what has to hold
//! for their output to stay byte-identical.

#[path = "../benches/fixture.rs"]
mod fixture;

use std::collections::HashSet;

use polymarket_substreams_common::{
    block_logs, block_timestamp_seconds, collect_user_addresses, format_address, tx_context,
    user_addresses_for_log, with_lazy_block, USER_ACTIVITY_CONTRACTS,
};
use substreams_ethereum::pb::eth::v2 as eth;

/// (tx_hash, log_index, block_number, timestamp, address, topics, data)
type Row = (String, u64, u64, u64, Vec<u8>, Vec<Vec<u8>>, Vec<u8>);

fn keep(address: &[u8]) -> bool {
    USER_ACTIVITY_CONTRACTS.iter().any(|c| address == c) || address[0].is_multiple_of(3)
}

fn owned_rows(blk: &eth::Block) -> Vec<Row> {
    blk.logs()
        .filter(|l| keep(&l.log.address))
        .map(|l| {
            (
                format_address(&l.receipt.transaction.hash),
                l.log.block_index as u64,
                blk.number,
                blk.timestamp_seconds(),
                l.log.address.clone(),
                l.log.topics.clone(),
                l.log.data.clone(),
            )
        })
        .collect()
}

fn lazy_rows(blk: &eth::BlockLazyView) -> Vec<Row> {
    let kept = block_logs(blk, keep).expect("fixture decodes");
    kept.logs
        .iter()
        .map(|l| {
            let ctx = tx_context(l.tx_hash, &l.log, kept.number, kept.timestamp);
            (
                ctx.tx_hash,
                ctx.log_index,
                ctx.block_number,
                ctx.timestamp,
                l.log.address.to_vec(),
                l.log.topics.iter().map(|t| t.to_vec()).collect(),
                l.log.data.to_vec(),
            )
        })
        .collect()
}

#[test]
fn block_logs_matches_owned_logs() {
    let blk = fixture::block();
    let owned = owned_rows(&blk);
    let lazy = with_lazy_block(&blk, lazy_rows).expect("fixture encodes");

    // The fixture must exercise both the address filter and failed transactions.
    let all_owned = blk.logs().count();
    assert!(owned.len() > 50 && owned.len() < all_owned);
    let all_raw: usize = blk
        .transaction_traces
        .iter()
        .map(|t| t.receipt.logs.len())
        .sum();
    assert!(
        all_owned < all_raw,
        "fixture must contain failed transactions"
    );

    assert_eq!(owned, lazy);
}

#[test]
fn collect_user_addresses_matches_owned_path() {
    let blk = fixture::block();
    let mut set = HashSet::new();
    for l in blk.logs() {
        for a in user_addresses_for_log(&l.log.address, l.log) {
            set.insert(a);
        }
    }
    let mut owned: Vec<Vec<u8>> = set.into_iter().collect();
    owned.sort();
    assert!(!owned.is_empty());

    let lazy = with_lazy_block(&blk, collect_user_addresses)
        .expect("fixture encodes")
        .expect("fixture decodes");
    assert_eq!(owned, lazy);
}

#[test]
fn timestamp_matches_owned_and_is_skipped_when_nothing_is_kept() {
    let blk = fixture::block();
    let ts = with_lazy_block(&blk, block_timestamp_seconds)
        .expect("fixture encodes")
        .expect("header present");
    assert_eq!(ts, blk.timestamp_seconds());

    // A headerless block with nothing kept is fine (the owned path never read
    // the header either) ...
    let empty = eth::Block::default();
    let kept = with_lazy_block(&empty, |b| block_logs(b, |_| true).map(|k| k.logs.len()))
        .expect("encodes")
        .expect("no header needed");
    assert_eq!(kept, 0);
    // ... while asking for its timestamp is an error, not a panic.
    assert!(with_lazy_block(&empty, block_timestamp_seconds)
        .expect("encodes")
        .is_err());
}

#[test]
fn successful_transaction_without_receipt_contributes_no_logs() {
    let mut blk = fixture::block();
    blk.transaction_traces[0].receipt = Default::default();
    let expected = fixture::block()
        .logs()
        .filter(|l| l.receipt.transaction.index != 0)
        .count();
    let got = with_lazy_block(&blk, |b| block_logs(b, |_| true).map(|k| k.logs.len()))
        .expect("encodes")
        .expect("decodes");
    assert_eq!(got, expected);
}
