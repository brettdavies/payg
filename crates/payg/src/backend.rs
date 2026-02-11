#[cfg(feature = "x402")]
pub mod x402;

#[cfg(feature = "eth")]
pub mod eth;

#[cfg(feature = "x402")]
use self::x402::X402Backend;

#[cfg(feature = "eth")]
use self::eth::EthBackend;

use alloy_signer_local::PrivateKeySigner;

use crate::charge::{ChargeReceipt, ChargeRequest};
use crate::config::ConsumerConfig;
use crate::error::PaygError;
use crate::network::NetworkConfig;

/// Payment backend, selected at compile time via feature flags.
pub enum Backend {
    #[cfg(feature = "x402")]
    X402(X402Backend),
    #[cfg(feature = "eth")]
    Eth(EthBackend),
}

impl Backend {
    /// Construct the appropriate backend from config.
    ///
    /// With default features (`x402`), this creates an X402 backend.
    /// If only `eth` is enabled, creates an ETH backend.
    pub fn from_config(
        consumer: &ConsumerConfig,
        network: &'static NetworkConfig,
        signer: PrivateKeySigner,
    ) -> Result<Self, PaygError> {
        #[cfg(feature = "x402")]
        {
            Ok(Self::X402(X402Backend::new(consumer, network, signer)?))
        }

        #[cfg(all(feature = "eth", not(feature = "x402")))]
        {
            Ok(Self::Eth(EthBackend::new(consumer, network, signer)?))
        }
    }

    /// Execute a charge against the selected backend.
    pub async fn charge(&self, request: &ChargeRequest) -> Result<ChargeReceipt, PaygError> {
        match self {
            #[cfg(feature = "x402")]
            Self::X402(b) => b.charge(request).await,
            #[cfg(feature = "eth")]
            Self::Eth(b) => b.charge(request).await,
        }
    }
}
