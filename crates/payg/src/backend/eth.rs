use std::time::Duration;

use alloy_network::EthereumWallet;
use alloy_provider::{Provider, ProviderBuilder};
use alloy_rpc_types_eth::TransactionRequest;
use alloy_signer_local::PrivateKeySigner;

use crate::charge::{ChargeReceipt, ChargeRequest};
use crate::config::ConsumerConfig;
use crate::error::PaygError;
use crate::network::NetworkConfig;

/// Direct ETH transfer backend using alloy-rs on Base L2.
pub struct EthBackend {
    rpc_url: String,
    signer: PrivateKeySigner,
}

impl EthBackend {
    pub fn new(
        config: &ConsumerConfig,
        network: &NetworkConfig,
        signer: PrivateKeySigner,
    ) -> Result<Self, PaygError> {
        Ok(Self {
            rpc_url: config.rpc_url(network)?,
            signer,
        })
    }

    pub async fn charge(&self, request: &ChargeRequest) -> Result<ChargeReceipt, PaygError> {
        let wallet = EthereumWallet::from(self.signer.clone());

        let provider = ProviderBuilder::new()
            .wallet(wallet)
            .connect(&self.rpc_url)
            .await
            .map_err(|e| PaygError::ConfigError(format!("failed to connect to RPC: {e}")))?;

        let tx = TransactionRequest::default()
            .to(request.recipient)
            .value(request.amount)
            .gas_limit(21_000);

        tracing::info!(
            recipient = %request.recipient,
            amount = %request.amount,
            "sending ETH transfer"
        );

        let pending = provider
            .send_transaction(tx)
            .await
            .map_err(|e| PaygError::PaymentFailed(format!("transaction send failed: {e}")))?;

        let tx_hash = format!("{:?}", pending.tx_hash());
        tracing::info!(%tx_hash, "transaction submitted, awaiting confirmation");

        let receipt = pending
            .with_timeout(Some(Duration::from_secs(60)))
            .get_receipt()
            .await
            .map_err(|e| PaygError::PaymentFailed(format!("receipt timeout: {e}")))?;

        let tx_hash = format!("{:?}", receipt.transaction_hash);
        tracing::info!(%tx_hash, "transaction confirmed");

        Ok(ChargeReceipt { tx_hash })
    }
}
