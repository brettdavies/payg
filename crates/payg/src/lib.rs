#[cfg(not(any(feature = "x402", feature = "eth")))]
compile_error!("At least one payment backend feature must be enabled: 'x402' or 'eth'");

pub mod backend;
pub mod charge;
pub mod config;
pub mod error;
pub mod pricing;
pub mod wallet;

pub use charge::{ChargeReceipt, ChargeRequest};
pub use config::{ConsumerConfig, ProjectConfig};
pub use error::PaygError;

use alloy_primitives::Address;
use backend::Backend;

/// Base L2 chain ID.
pub const BASE_CHAIN_ID: u64 = 8453;

/// USDC contract address on Base.
pub const BASE_USDC_ADDRESS: &str = "0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913";

/// Default safety ceiling: $1.00 USDC (6 decimals).
pub const DEFAULT_SAFETY_CEILING_USDC: u64 = 1_000_000;

/// Default facilitator URL.
pub const DEFAULT_FACILITATOR_URL: &str = "https://x402.org/facilitator";

/// Charge a recipient with the given amount string (e.g. "0.001 USDC").
///
/// This is the primary public API. It loads config, parses the amount,
/// checks the safety ceiling, and dispatches to the appropriate backend.
pub async fn charge(amount: &str, recipient: &str) -> Result<ChargeReceipt, PaygError> {
    let consumer_config = ConsumerConfig::load()?;
    let project_config = ProjectConfig::load().ok();

    let parsed = pricing::parse_price(amount)?;
    let recipient: Address = recipient
        .parse()
        .map_err(|e| PaygError::ConfigError(format!("invalid recipient address: {e}")))?;

    let ceiling = consumer_config.safety_ceiling_u256();
    if parsed.amount > ceiling {
        return Err(PaygError::ExceedsSafetyCeiling {
            amount: amount.to_string(),
            ceiling: consumer_config
                .max_charge
                .clone()
                .unwrap_or_else(|| "1.00 USDC".to_string()),
        });
    }

    let request = ChargeRequest {
        amount: parsed.amount,
        recipient,
    };

    let signer = wallet::load_wallet(&consumer_config)?;
    let backend = Backend::from_config(&consumer_config, &project_config, signer)?;
    backend.charge(&request).await
}
