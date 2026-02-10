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
use alloy_signer_local::PrivateKeySigner;
use backend::Backend;

/// Base L2 chain ID.
pub const BASE_CHAIN_ID: u64 = 8453;

/// USDC contract address on Base.
pub const BASE_USDC_ADDRESS: &str = "0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913";

/// Default safety ceiling: $1.00 USDC (6 decimals).
pub const DEFAULT_SAFETY_CEILING_USDC: u64 = 1_000_000;

/// Default safety ceiling display string.
pub const DEFAULT_SAFETY_CEILING_DISPLAY: &str = "1.00 USDC";

/// Default facilitator URL.
pub const DEFAULT_FACILITATOR_URL: &str = "https://x402.org/facilitator";

/// Charge a recipient with the given amount string (e.g. "0.001 USDC").
///
/// This is the convenience API that loads config and wallet automatically.
/// For pre-loaded config/wallet, use [`charge_with_config`].
pub async fn charge(amount: &str, recipient: &str) -> Result<ChargeReceipt, PaygError> {
    let consumer_config = ConsumerConfig::load()?;
    let password = std::env::var("PAYG_KEY_PASSWORD").ok();
    let signer = wallet::load_wallet(&consumer_config, password.as_deref())?;
    charge_with_config(amount, recipient, &consumer_config, signer).await
}

/// Charge a recipient using pre-loaded config and signer.
///
/// Use this when you already have the config and signer loaded (avoids redundant
/// config file reads and keyfile decryption).
pub async fn charge_with_config(
    amount: &str,
    recipient: &str,
    consumer_config: &ConsumerConfig,
    signer: PrivateKeySigner,
) -> Result<ChargeReceipt, PaygError> {
    let parsed = pricing::parse_price(amount)?;
    let recipient: Address = recipient
        .parse()
        .map_err(|e| PaygError::ConfigError(format!("invalid recipient address: {e}")))?;

    check_safety_ceiling(&parsed, amount, consumer_config)?;

    let request = ChargeRequest {
        amount: parsed.amount,
        recipient,
    };

    let backend = Backend::from_config(consumer_config, signer)?;
    backend.charge(&request).await
}

/// Check that a parsed price does not exceed the safety ceiling.
///
/// The ceiling and charge must use the same token. If the ceiling is in USDC
/// and the charge is in ETH (or vice versa), the check returns an error.
pub fn check_safety_ceiling(
    parsed: &pricing::ParsedPrice,
    original_amount: &str,
    config: &ConsumerConfig,
) -> Result<(), PaygError> {
    let (ceiling, ceiling_token) = config.safety_ceiling_with_token();

    if parsed.token != ceiling_token {
        return Err(PaygError::ConfigError(format!(
            "safety ceiling is in {ceiling_token} but charge is in {} — set PAYG_MAX_CHARGE in {} units",
            parsed.token, parsed.token
        )));
    }

    if parsed.amount > ceiling {
        return Err(PaygError::ExceedsSafetyCeiling {
            amount: original_amount.to_string(),
            ceiling: config
                .max_charge
                .clone()
                .unwrap_or_else(|| DEFAULT_SAFETY_CEILING_DISPLAY.to_string()),
        });
    }

    Ok(())
}
