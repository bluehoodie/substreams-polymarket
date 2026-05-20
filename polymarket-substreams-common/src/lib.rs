use substreams::scalar::BigInt;
use substreams_ethereum::pb::eth::v2 as eth;
use substreams_ethereum::block_view::LogView;

#[inline]
pub fn bigint_to_string(bigint: &BigInt) -> String {
    let s = bigint.to_string();
    if s.is_empty() { "0".to_string() } else { s }
}

#[inline]
pub fn bigint_to_u32(bigint: &BigInt) -> u32 {
    bigint.to_u64() as u32
}

#[inline]
pub fn format_address(bytes: &[u8]) -> String {
    format!("0x{}", hex::encode(bytes))
}

/// Primitive transaction-context fields, package-agnostic.
/// Each substreams package wraps this into its own `proto::TransactionContext`.
pub struct TxContext {
    pub tx_hash: String,
    pub log_index: u64,
    pub block_number: u64,
    pub timestamp: u64,
}

#[inline]
pub fn build_tx_context(blk: &eth::Block, log: &LogView) -> TxContext {
    TxContext {
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
    fn test_bigint_to_string_zero() {
        let zero = BigInt::from(0u64);
        // BigInt::from(0) gives "0", not empty — still correct output
        assert_eq!(bigint_to_string(&zero), "0");
    }

    #[test]
    fn test_bigint_to_string_normal() {
        let val = BigInt::from(12345u64);
        assert_eq!(bigint_to_string(&val), "12345");
    }

    #[test]
    fn test_bigint_to_string_large() {
        let val = BigInt::from(u64::MAX);
        assert_eq!(bigint_to_string(&val), u64::MAX.to_string());
    }

    #[test]
    fn test_format_address_known() {
        let bytes: [u8; 4] = [0xde, 0xad, 0xbe, 0xef];
        assert_eq!(format_address(&bytes), "0xdeadbeef");
    }

    #[test]
    fn test_format_address_zero() {
        let bytes: [u8; 4] = [0x00, 0x00, 0x00, 0x00];
        assert_eq!(format_address(&bytes), "0x00000000");
    }

    #[test]
    fn test_bigint_to_u32_zero() {
        let val = BigInt::from(0u64);
        assert_eq!(bigint_to_u32(&val), 0u32);
    }

    #[test]
    fn test_bigint_to_u32_one() {
        let val = BigInt::from(1u64);
        assert_eq!(bigint_to_u32(&val), 1u32);
    }

    #[test]
    fn test_bigint_to_u32_max() {
        let val = BigInt::from(u32::MAX as u64);
        assert_eq!(bigint_to_u32(&val), u32::MAX);
    }
}
