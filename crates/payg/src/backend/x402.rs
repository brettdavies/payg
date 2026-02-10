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
use crate::{BASE_CHAIN_ID, BASE_USDC_ADDRESS};

/// x402 payment backend using ERC-3009 transferWithAuthorization via USDC on Base.
pub struct X402Backend {
    facilitator_url: String,
    signer: PrivateKeySigner,
    usdc_address: Address,
}

impl X402Backend {
    pub fn new(config: &ConsumerConfig, signer: PrivateKeySigner) -> Self {
        Self {
            facilitator_url: config.facilitator_url(),
            signer,
            usdc_address: BASE_USDC_ADDRESS
                .parse()
                .expect("hardcoded USDC address is valid"),
        }
    }

    pub async fn charge(&self, request: &ChargeRequest) -> Result<ChargeReceipt, PaygError> {
        // 1. Sign EIP-3009 authorization
        let params = Eip3009SigningParams {
            chain_id: BASE_CHAIN_ID,
            asset_address: self.usdc_address,
            pay_to: request.recipient,
            amount: request.amount,
            max_timeout_seconds: 3600,
            extra: Some(PaymentRequirementsExtra {
                name: "USD Coin".to_string(),
                version: "2".to_string(),
            }),
        };

        let payload = sign_erc3009_authorization(&self.signer, &params)
            .await
            .map_err(|e| PaygError::PaymentFailed(format!("signing failed: {e}")))?;

        // 2. Build the V1 settle request
        let settle_request = build_settle_request(
            payload,
            request.amount,
            request.recipient,
            self.usdc_address,
        );

        let settle_json = serde_json::to_value(&settle_request)?;

        tracing::info!(
            facilitator = %self.facilitator_url,
            recipient = %request.recipient,
            "submitting payment to facilitator"
        );

        // 3. POST to facilitator /settle
        let client = reqwest::Client::new();
        let response = client
            .post(format!("{}/settle", self.facilitator_url))
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
                .unwrap_or("unknown")
                .to_string();

            tracing::info!(%tx_hash, "payment settled on-chain");
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

/// Build the V1 VerifyRequest (which is also the SettleRequest).
fn build_settle_request(
    payload: ExactEvmPayload,
    amount: U256,
    recipient: Address,
    asset: Address,
) -> VerifyRequest<PaymentPayload<ExactScheme, ExactEvmPayload>, PaymentRequirements> {
    let payment_payload = PaymentPayload {
        x402_version: X402Version1,
        scheme: ExactScheme,
        network: "base".to_string(),
        payload,
    };

    let payment_requirements = PaymentRequirements {
        scheme: ExactScheme,
        network: "base".to_string(),
        max_amount_required: amount,
        resource: "payg://charge".to_string(),
        description: "PAYG CLI charge".to_string(),
        mime_type: "application/json".to_string(),
        output_schema: None,
        pay_to: recipient,
        max_timeout_seconds: 3600,
        asset,
        extra: Some(PaymentRequirementsExtra {
            name: "USD Coin".to_string(),
            version: "2".to_string(),
        }),
    };

    VerifyRequest {
        x402_version: X402Version1,
        payment_payload,
        payment_requirements,
    }
}
