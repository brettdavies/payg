use alloy_signer_local::PrivateKeySigner;
use zeroize::Zeroizing;

use crate::config::ConsumerConfig;
use crate::error::PaygError;

/// Load a wallet signer from environment or encrypted keyfile.
///
/// Checks in order:
/// 1. `PAYG_PRIVATE_KEY` env var (recommended for agents — avoids scrypt overhead)
/// 2. Encrypted keyfile at the configured path (default: `~/.payg/keyfile.json`)
///
/// For the keyfile path, a password is required. The caller provides it via
/// the `password` parameter — typically from `PAYG_KEY_PASSWORD` env var or
/// an interactive prompt in the CLI.
pub fn load_wallet(
    config: &ConsumerConfig,
    password: Option<&str>,
) -> Result<PrivateKeySigner, PaygError> {
    // Fast path: env var (agents, CI, testing)
    if let Ok(key) = std::env::var("PAYG_PRIVATE_KEY") {
        tracing::debug!("loading wallet from PAYG_PRIVATE_KEY env var");
        let key = Zeroizing::new(key);
        return key
            .parse::<PrivateKeySigner>()
            .map_err(|e| PaygError::WalletError(format!("invalid private key: {e}")));
    }

    // Slow path: encrypted keyfile (200-500ms scrypt decryption)
    let keyfile = config.keyfile_path()?;
    if !keyfile.exists() {
        return Err(PaygError::NoWallet);
    }

    let password = password.ok_or_else(|| {
        PaygError::WalletError(
            "keyfile password required. Set PAYG_PRIVATE_KEY or PAYG_KEY_PASSWORD environment variable for non-interactive use".to_string()
        )
    })?;

    tracing::debug!(?keyfile, "decrypting keyfile");

    PrivateKeySigner::decrypt_keystore(&keyfile, password)
        .map_err(|e| PaygError::WalletError(format!("keyfile decryption failed: {e}")))
}
