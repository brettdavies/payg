use alloy_primitives::{Address, U256};
use serde::Serialize;

/// A parsed charge request ready for backend dispatch.
#[derive(Debug, Clone)]
pub struct ChargeRequest {
    pub amount: U256,
    pub recipient: Address,
}

/// Receipt returned after a successful charge.
#[derive(Debug, Clone, Serialize)]
pub struct ChargeReceipt {
    pub tx_hash: String,
}
