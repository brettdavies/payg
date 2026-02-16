//! x402 backend — ERC-3009 USDC transfers via facilitator.
//!
//! Signs an ERC-3009 `transferWithAuthorization` locally, then POSTs the
//! signed payload to an x402 facilitator which submits the on-chain transaction.
//! The facilitator pays gas; the signer only needs USDC balance.

use std::time::Duration;

use alloy_primitives::{Address, U256};
use alloy_signer_local::PrivateKeySigner;
use x402_chain_eip155::v1_eip155_exact::client::{
    Eip3009SigningParams, sign_erc3009_authorization,
};
use x402_chain_eip155::v1_eip155_exact::types::{
    ExactEvmPayload, ExactScheme, PaymentRequirements, PaymentRequirementsExtra,
};
use x402_types::proto::v1::{PaymentPayload, VerifyRequest, X402Version1};

use crate::charge::{ChargeReceipt, ChargeRequest};
use crate::config::ConsumerConfig;
use crate::error::PaygError;
use crate::network::NetworkConfig;

/// Maximum authorization validity window in seconds (2 minutes).
const MAX_TIMEOUT_SECONDS: u64 = 120;

/// x402 payment backend using ERC-3009 transferWithAuthorization via USDC on Base.
pub struct X402Backend {
    facilitator_url: String,
    signer: PrivateKeySigner,
    usdc_address: Address,
    client: reqwest::Client,
    network: &'static NetworkConfig,
}

impl X402Backend {
    pub fn new(
        config: &ConsumerConfig,
        network: &'static NetworkConfig,
        signer: PrivateKeySigner,
    ) -> Result<Self, PaygError> {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .connect_timeout(Duration::from_secs(10))
            .build()
            .map_err(|e| PaygError::Http(e.to_string()))?;

        let usdc_address: Address = network
            .usdc_address
            .parse()
            .map_err(|e| PaygError::ConfigError(format!("invalid USDC address: {e}")))?;

        Ok(Self {
            facilitator_url: config.facilitator_url()?,
            signer,
            usdc_address,
            client,
            network,
        })
    }

    pub async fn charge(&self, request: &ChargeRequest) -> Result<ChargeReceipt, PaygError> {
        // 1. Sign EIP-3009 authorization
        let params = Eip3009SigningParams {
            chain_id: self.network.chain_id,
            asset_address: self.usdc_address,
            pay_to: request.recipient,
            amount: request.amount,
            max_timeout_seconds: MAX_TIMEOUT_SECONDS,
            extra: eip712_extra(self.network),
        };

        let payload = sign_erc3009_authorization(&self.signer, &params)
            .await
            .map_err(|e| PaygError::PaymentFailed(format!("signing failed: {e}")))?;

        // 2. Build the V1 settle request
        let settle_request = build_settle_request(payload, request.amount, request.recipient, self);

        let settle_json = serde_json::to_value(&settle_request)?;

        tracing::info!(
            facilitator = %self.facilitator_url,
            recipient = %request.recipient,
            network = %self.network.name,
            "submitting payment to facilitator"
        );

        // 3. POST to facilitator /settle
        let response = self
            .client
            .post(format!(
                "{}/settle",
                self.facilitator_url.trim_end_matches('/')
            ))
            .json(&settle_json)
            .send()
            .await
            .map_err(|e| PaygError::Http(e.to_string()))?;

        let status = response.status();
        let body: serde_json::Value = response
            .json()
            .await
            .map_err(|e| PaygError::Http(e.to_string()))?;

        // 4. Parse settle response
        if status.is_success()
            && let Some(true) = body.get("success").and_then(|v| v.as_bool())
        {
            let tx_hash = body
                .get("transaction")
                .and_then(|v| v.as_str())
                .filter(|h| h.starts_with("0x") && h.len() == 66)
                .ok_or_else(|| {
                    PaygError::PaymentFailed(
                        "facilitator returned success but invalid or missing tx_hash".to_string(),
                    )
                })?
                .to_string();

            tracing::info!(%tx_hash, "payment settled on-chain (unverified)");
            return Ok(ChargeReceipt { tx_hash });
        }

        let reason = body
            .get("errorReason")
            .or_else(|| body.get("error_reason"))
            .and_then(|v| v.as_str())
            .unwrap_or("unknown error");

        Err(PaygError::PaymentFailed(format!(
            "facilitator rejected payment: {reason}"
        )))
    }
}

/// Build the EIP-712 extra parameters from a network config.
fn eip712_extra(network: &NetworkConfig) -> Option<PaymentRequirementsExtra> {
    Some(PaymentRequirementsExtra {
        name: network.eip712_name.to_string(),
        version: network.eip712_version.to_string(),
    })
}

/// Build the V1 VerifyRequest (which is also the SettleRequest).
fn build_settle_request(
    payload: ExactEvmPayload,
    amount: U256,
    recipient: Address,
    backend: &X402Backend,
) -> VerifyRequest<PaymentPayload<ExactScheme, ExactEvmPayload>, PaymentRequirements> {
    let payment_payload = PaymentPayload {
        x402_version: X402Version1,
        scheme: ExactScheme,
        network: backend.network.name.to_string(),
        payload,
    };

    let payment_requirements = PaymentRequirements {
        scheme: ExactScheme,
        network: backend.network.name.to_string(),
        max_amount_required: amount,
        resource: "payg://charge".to_string(),
        description: "PAYG CLI charge".to_string(),
        mime_type: "application/json".to_string(),
        output_schema: None,
        pay_to: recipient,
        max_timeout_seconds: MAX_TIMEOUT_SECONDS,
        asset: backend.usdc_address,
        extra: eip712_extra(backend.network),
    };

    VerifyRequest {
        x402_version: X402Version1,
        payment_payload,
        payment_requirements,
    }
}
