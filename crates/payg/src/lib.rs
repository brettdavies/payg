#[cfg(not(any(feature = "x402", feature = "eth")))]
compile_error!("At least one payment backend feature must be enabled: 'x402' or 'eth'");

pub mod backend;
pub mod charge;
pub mod config;
pub mod error;
pub mod network;
pub mod pricing;
pub mod wallet;

pub use charge::{ChargeReceipt, ChargeRequest};
pub use config::{ConsumerConfig, ProjectConfig};
pub use error::PaygError;
pub use network::NetworkConfig;

use alloy_primitives::Address;
use alloy_signer_local::PrivateKeySigner;
use backend::Backend;

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
    let network = consumer_config.resolve_network()?;
    let password = std::env::var("PAYG_KEY_PASSWORD").ok();
    let signer = wallet::load_wallet(&consumer_config, password.as_deref())?;
    charge_with_config(amount, recipient, &consumer_config, network, signer).await
}

/// Charge a recipient using pre-loaded config and signer.
///
/// Parses and validates the amount and recipient, then executes the charge.
/// For pre-validated inputs (e.g. when the CLI has already validated), use
/// `charge_raw` to avoid redundant parsing.
pub async fn charge_with_config(
    amount: &str,
    recipient: &str,
    consumer_config: &ConsumerConfig,
    network: &'static NetworkConfig,
    signer: PrivateKeySigner,
) -> Result<ChargeReceipt, PaygError> {
    let parsed = pricing::parse_price(amount)?;
    let recipient: Address = recipient
        .parse()
        .map_err(|e| PaygError::ConfigError(format!("invalid recipient address: {e}")))?;

    check_safety_ceiling(&parsed, amount, consumer_config)?;

    charge_raw(parsed.amount, recipient, consumer_config, network, signer).await
}

/// Charge with pre-parsed amount and recipient, skipping validation.
///
/// # Safety (not `unsafe`, but caller-beware)
///
/// This function skips amount parsing and safety ceiling checks. The caller
/// is responsible for validating the amount against the safety ceiling before
/// calling this function. Prefer [`charge_with_config`] for string inputs
/// with automatic validation.
pub async fn charge_raw(
    amount: alloy_primitives::U256,
    recipient: Address,
    consumer_config: &ConsumerConfig,
    network: &'static NetworkConfig,
    signer: PrivateKeySigner,
) -> Result<ChargeReceipt, PaygError> {
    let request = ChargeRequest { amount, recipient };
    let backend = Backend::from_config(consumer_config, network, signer)?;
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
