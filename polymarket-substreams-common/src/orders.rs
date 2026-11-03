//! Signed orders from exchange settlement calldata.
//!
//! `OrderFilled` only carries what was filled. The full signed order (its total
//! maker and taker amounts, so its limit price and size) is in the input of the
//! operator's `matchOrders` call. This module decodes those calls and computes each
//! order's EIP-712 hash, the same `orderHash` the exchange puts in `OrderFilled`.
//!
//! Settlement entry points (checked against live calls on blocks 96600000 and
//! 96680009, where every one is a top-level call from an operator EOA):
//! - CTF Exchange V2 and Neg Risk CTF Exchange V2 (ctf-exchange-v2
//!   `src/exchange/CTFExchange.sol`): `matchOrders(bytes32,Order,Order[],uint256,
//!   uint256[],uint256,uint256[])`, selector `0x3c2b4399`. EIP-712 domain
//!   "Polymarket CTF Exchange" version "2".
//! - ExchangeV3 (polymarket-v2-external `src/exchange/Exchange.sol`):
//!   `matchOrders(Order,Order[],uint256[],uint256[],TakerAmounts)` (`0x0fb25ba4`)
//!   and `matchOrdersAndPrepareCombinatorial(…, uint256[])` (`0xe75cf6e5`). Domain
//!   "Polymarket CTF Exchange" version "3", on the proxy address.
//!
//! All three share one `Order` struct and type hash.

use sha3::{Digest, Keccak256};
use substreams::scalar::BigInt;
use substreams_ethereum::pb::eth::v2 as eth;

/// `matchOrders` on CTF Exchange V2 / Neg Risk CTF Exchange V2.
pub const V2_MATCH_ORDERS: [u8; 4] = hex_literal::hex!("3c2b4399");
/// `matchOrders` on ExchangeV3.
pub const V3_MATCH_ORDERS: [u8; 4] = hex_literal::hex!("0fb25ba4");
/// `matchOrdersAndPrepareCombinatorial` on ExchangeV3.
pub const V3_MATCH_ORDERS_COMBINATORIAL: [u8; 4] = hex_literal::hex!("e75cf6e5");

use crate::abi::{ctf_exchange_v2_match as v2, exchange_v3_match as v3};
use crate::{bigint_to_u32, CTF_EXCHANGE, EXCHANGE_V3, NEG_RISK_CTF};

/// Polygon PoS chain id, part of every exchange's EIP-712 domain.
pub const POLYGON_CHAIN_ID: u64 = 137;

/// keccak256("Order(uint256 salt,address maker,address signer,uint256 tokenId,
/// uint256 makerAmount,uint256 takerAmount,uint8 side,uint8 signatureType,
/// uint256 timestamp,bytes32 metadata,bytes32 builder)")
pub const ORDER_TYPEHASH: [u8; 32] =
    hex_literal::hex!("bb86318a2138f5fa8ae32fbe8e659f8fcf13cc6ae4014a707893055433818589");

const EIP712_DOMAIN_TYPEHASH_PREIMAGE: &[u8] =
    b"EIP712Domain(string name,string version,uint256 chainId,address verifyingContract)";

/// An exchange whose settlement calls this module decodes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Exchange {
    pub address: [u8; 20],
    /// Same labels as `Trade.exchange`: `ctf_v2`, `neg_risk_v2`, `exchange_v3`.
    pub name: &'static str,
    domain_name: &'static str,
    domain_version: &'static str,
}

pub const EXCHANGES: [Exchange; 3] = [
    Exchange {
        address: CTF_EXCHANGE,
        name: "ctf_v2",
        domain_name: "Polymarket CTF Exchange",
        domain_version: "2",
    },
    Exchange {
        address: NEG_RISK_CTF,
        name: "neg_risk_v2",
        domain_name: "Polymarket CTF Exchange",
        domain_version: "2",
    },
    Exchange {
        address: EXCHANGE_V3,
        name: "exchange_v3",
        domain_name: "Polymarket CTF Exchange",
        domain_version: "3",
    },
];

/// The exchange at `address`, if it is one of the three.
pub fn exchange_at(address: &[u8]) -> Option<&'static Exchange> {
    EXCHANGES.iter().find(|e| address == e.address)
}

impl Exchange {
    /// EIP-712 domain separator (solady `EIP712`, chain 137, this address).
    pub fn domain_separator(&self) -> [u8; 32] {
        let mut buf = Vec::with_capacity(5 * 32);
        buf.extend_from_slice(&keccak(EIP712_DOMAIN_TYPEHASH_PREIMAGE));
        buf.extend_from_slice(&keccak(self.domain_name.as_bytes()));
        buf.extend_from_slice(&keccak(self.domain_version.as_bytes()));
        buf.extend_from_slice(&u64_word(POLYGON_CHAIN_ID));
        buf.extend_from_slice(&address_word(&self.address));
        keccak(&buf)
    }
}

/// One signed order from a settlement call, with the amounts the operator filled.
#[derive(Clone, Debug, PartialEq)]
pub struct SettledOrder {
    /// EIP-712 order hash, as in `OrderFilled.orderHash`.
    pub hash: [u8; 32],
    /// The taker order of the match (the rest are maker orders).
    pub is_taker: bool,
    pub salt: BigInt,
    pub maker: Vec<u8>,
    pub signer: Vec<u8>,
    pub token_id: BigInt,
    /// Full size of the order: the most the maker gives, and the least they get.
    pub maker_amount: BigInt,
    pub taker_amount: BigInt,
    /// 0 = BUY, 1 = SELL.
    pub side: u32,
    /// 0 EOA, 1 POLY_PROXY, 2 POLY_GNOSIS_SAFE, 3 POLY_1271.
    pub signature_type: u32,
    /// Order creation time in milliseconds.
    pub timestamp_ms: BigInt,
    pub metadata: [u8; 32],
    pub builder: [u8; 32],
    /// Maker-asset amount this call fills, and the fee charged on it.
    pub fill_amount: BigInt,
    pub fee_amount: BigInt,
}

type OrderTuple = (
    BigInt,
    Vec<u8>,
    Vec<u8>,
    BigInt,
    BigInt,
    BigInt,
    BigInt,
    BigInt,
    BigInt,
    [u8; 32],
    [u8; 32],
    Vec<u8>,
);

/// Decodes a settlement call to `exchange` and returns its orders, taker first,
/// then makers in call order. `None` when the input is not a settlement call or
/// fails to decode.
pub fn decode_settlement(exchange: &Exchange, input: &[u8]) -> Option<Vec<SettledOrder>> {
    let selector = input.get(..4)?;
    // The generated decoders take an owned `Call`; build one around the input only
    // after the selector matched, so other calls cost no copy.
    let call = |input: &[u8]| eth::Call {
        input: input.to_vec(),
        ..Default::default()
    };
    let (taker, makers, taker_fill, maker_fills, taker_fee, maker_fees) = if exchange.address
        == EXCHANGE_V3
    {
        if selector == V3_MATCH_ORDERS {
            let m = v3::functions::MatchOrders::decode(&call(input)).ok()?;
            let (fill, _receive, fee) = m.taker_amounts;
            (
                m.taker_order,
                m.maker_orders,
                fill,
                m.maker_fill_amounts,
                fee,
                m.maker_fee_amounts,
            )
        } else if selector == V3_MATCH_ORDERS_COMBINATORIAL {
            let m = v3::functions::MatchOrdersAndPrepareCombinatorial::decode(&call(input)).ok()?;
            let (fill, _receive, fee) = m.taker_amounts;
            (
                m.taker_order,
                m.maker_orders,
                fill,
                m.maker_fill_amounts,
                fee,
                m.maker_fee_amounts,
            )
        } else {
            return None;
        }
    } else if selector == V2_MATCH_ORDERS {
        let m = v2::functions::MatchOrders::decode(&call(input)).ok()?;
        (
            m.taker_order,
            m.maker_orders,
            m.taker_fill_amount,
            m.maker_fill_amounts,
            m.taker_fee_amount,
            m.maker_fee_amounts,
        )
    } else {
        return None;
    };
    if makers.len() != maker_fills.len() || makers.len() != maker_fees.len() {
        return None;
    }
    let domain = exchange.domain_separator();
    let mut out = Vec::with_capacity(1 + makers.len());
    out.push(settled(&domain, taker, true, taker_fill, taker_fee));
    for ((o, fill), fee) in makers.into_iter().zip(maker_fills).zip(maker_fees) {
        out.push(settled(&domain, o, false, fill, fee));
    }
    Some(out)
}

fn settled(
    domain: &[u8; 32],
    o: OrderTuple,
    is_taker: bool,
    fill_amount: BigInt,
    fee_amount: BigInt,
) -> SettledOrder {
    let (
        salt,
        maker,
        signer,
        token_id,
        maker_amount,
        taker_amount,
        side,
        sig,
        ts,
        metadata,
        builder,
        _sig,
    ) = o;
    let mut order = SettledOrder {
        hash: [0; 32],
        is_taker,
        salt,
        maker,
        signer,
        token_id,
        maker_amount,
        taker_amount,
        side: bigint_to_u32(&side),
        signature_type: bigint_to_u32(&sig),
        timestamp_ms: ts,
        metadata,
        builder,
        fill_amount,
        fee_amount,
    };
    order.hash = order_hash(domain, &order);
    order
}

/// EIP-712 hash of an order: keccak256(0x1901 ‖ domain ‖ structHash).
pub fn order_hash(domain_separator: &[u8; 32], o: &SettledOrder) -> [u8; 32] {
    let mut s = Vec::with_capacity(12 * 32);
    s.extend_from_slice(&ORDER_TYPEHASH);
    s.extend_from_slice(&uint_word(&o.salt));
    s.extend_from_slice(&address_word(&o.maker));
    s.extend_from_slice(&address_word(&o.signer));
    s.extend_from_slice(&uint_word(&o.token_id));
    s.extend_from_slice(&uint_word(&o.maker_amount));
    s.extend_from_slice(&uint_word(&o.taker_amount));
    s.extend_from_slice(&u64_word(o.side as u64));
    s.extend_from_slice(&u64_word(o.signature_type as u64));
    s.extend_from_slice(&uint_word(&o.timestamp_ms));
    s.extend_from_slice(&o.metadata);
    s.extend_from_slice(&o.builder);
    let struct_hash = keccak(&s);
    let mut d = Vec::with_capacity(66);
    d.extend_from_slice(&[0x19, 0x01]);
    d.extend_from_slice(domain_separator);
    d.extend_from_slice(&struct_hash);
    keccak(&d)
}

fn keccak(data: &[u8]) -> [u8; 32] {
    Keccak256::digest(data).into()
}

/// Big-endian 32-byte word of a non-negative integer; values over 256 bits keep
/// their low 32 bytes (calldata `uint256`s never are).
fn uint_word(v: &BigInt) -> [u8; 32] {
    let (_, bytes) = v.to_bytes_be();
    let mut w = [0u8; 32];
    let n = bytes.len().min(32);
    w[32 - n..].copy_from_slice(&bytes[bytes.len() - n..]);
    w
}

fn u64_word(v: u64) -> [u8; 32] {
    let mut w = [0u8; 32];
    w[24..].copy_from_slice(&v.to_be_bytes());
    w
}

fn address_word(a: &[u8]) -> [u8; 32] {
    let mut w = [0u8; 32];
    let n = a.len().min(20);
    w[32 - n..].copy_from_slice(&a[a.len() - n..]);
    w
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn order_typehash_matches_its_preimage() {
        let pre = b"Order(uint256 salt,address maker,address signer,uint256 tokenId,uint256 makerAmount,uint256 takerAmount,uint8 side,uint8 signatureType,uint256 timestamp,bytes32 metadata,bytes32 builder)";
        assert_eq!(keccak(pre), ORDER_TYPEHASH);
    }

    #[test]
    fn selectors_match_the_abi() {
        let call = |sel: [u8; 4]| eth::Call {
            input: sel.to_vec(),
            ..Default::default()
        };
        assert!(v2::functions::MatchOrders::match_call(&call(
            V2_MATCH_ORDERS
        )));
        assert!(v3::functions::MatchOrders::match_call(&call(
            V3_MATCH_ORDERS
        )));
        assert!(
            v3::functions::MatchOrdersAndPrepareCombinatorial::match_call(&call(
                V3_MATCH_ORDERS_COMBINATORIAL
            ))
        );
    }

    #[test]
    fn non_settlement_input_is_skipped() {
        let ex = &EXCHANGES[0];
        assert!(decode_settlement(ex, &[]).is_none());
        assert!(decode_settlement(ex, &hex_literal::hex!("3c2b4399")).is_none());
        assert!(decode_settlement(ex, &hex_literal::hex!("deadbeef00")).is_none());
    }
}
