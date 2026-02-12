use alloy_primitives::{Address, U256};
use serde::Serialize;

/// A parsed charge request ready for backend dispatch.
#[derive(Debug, Clone)]
pub struct ChargeRequest {
    /// Token amount in smallest units (e.g. 6-decimal USDC microdollars).
    pub amount: U256,
    /// On-chain recipient address.
    pub recipient: Address,
}

/// Receipt returned after a successful charge.
#[derive(Debug, Clone, Serialize)]
pub struct ChargeReceipt {
    /// Transaction hash (hex string with `0x` prefix, 66 chars).
    pub tx_hash: String,
}
