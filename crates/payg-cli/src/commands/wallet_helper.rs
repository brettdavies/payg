use alloy_signer_local::PrivateKeySigner;
use payg::{ConsumerConfig, PaygError};

/// Load wallet with automatic password resolution.
///
/// Password sources in order:
/// 1. PAYG_PRIVATE_KEY env var (skips password entirely)
/// 2. PAYG_KEY_PASSWORD env var
/// 3. Interactive TTY prompt via rpassword
pub fn load_wallet_interactive(config: &ConsumerConfig) -> Result<PrivateKeySigner, PaygError> {
    // Try env var password first
    let env_password = std::env::var("PAYG_KEY_PASSWORD").ok();
    if env_password.is_some() {
        return payg::wallet::load_wallet(config, env_password.as_deref());
    }

    // Try without password (works if PAYG_PRIVATE_KEY is set)
    match payg::wallet::load_wallet(config, None) {
        Ok(signer) => return Ok(signer),
        Err(PaygError::WalletError(_)) => {
            // Keyfile exists but needs password — fall through to prompt
        }
        Err(e) => return Err(e),
    }

    // Interactive prompt (only works with a TTY)
    let password = rpassword::prompt_password("Keyfile password: ").map_err(|e| {
        PaygError::WalletError(format!(
            "failed to read password: {e}. Set PAYG_PRIVATE_KEY or PAYG_KEY_PASSWORD environment variable for non-interactive use"
        ))
    })?;

    payg::wallet::load_wallet(config, Some(&password))
}
