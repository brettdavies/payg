use std::path::PathBuf;

use alloy_signer_local::PrivateKeySigner;
use clap::Args;

use payg::PaygError;

#[derive(Args)]
pub struct InitArgs {
    /// Run without interactive prompts (uses env vars)
    #[arg(long)]
    non_interactive: bool,
}

pub async fn run(args: InitArgs) -> Result<(), PaygError> {
    let home = std::env::var("HOME")
        .map(PathBuf::from)
        .map_err(|_| PaygError::ConfigError("HOME not set".to_string()))?;

    let payg_dir = home.join(".payg");
    std::fs::create_dir_all(&payg_dir)?;

    let keyfile_path = payg_dir.join("keyfile.json");
    if keyfile_path.exists() {
        return Err(PaygError::ConfigError(
            "wallet already exists at ~/.payg/keyfile.json — delete it first to reinitialize"
                .to_string(),
        ));
    }

    let password = if args.non_interactive {
        std::env::var("PAYG_KEY_PASSWORD").map_err(|_| {
            PaygError::ConfigError(
                "PAYG_KEY_PASSWORD must be set for non-interactive init".to_string(),
            )
        })?
    } else {
        let pw = rpassword::prompt_password("Set keyfile password: ")
            .map_err(|e| PaygError::WalletError(format!("failed to read password: {e}")))?;
        let confirm = rpassword::prompt_password("Confirm password: ")
            .map_err(|e| PaygError::WalletError(format!("failed to read password: {e}")))?;
        if pw != confirm {
            return Err(PaygError::WalletError("passwords do not match".to_string()));
        }
        pw
    };

    // Generate new key and encrypt to keyfile
    let signer = PrivateKeySigner::random();
    let pk_bytes = signer.credential().to_bytes();

    PrivateKeySigner::encrypt_keystore(
        &keyfile_path,
        &mut rand::thread_rng(),
        pk_bytes,
        &password,
        None,
    )
    .map_err(|e| PaygError::WalletError(format!("failed to create keyfile: {e}")))?;

    // Write default config
    let config_path = payg_dir.join("config.toml");
    if !config_path.exists() {
        let config = "keyfile = \"~/.payg/keyfile.json\"\nmax_charge = \"1.00 USDC\"\n".to_string();
        std::fs::write(&config_path, config)?;
    }

    // Set restrictive permissions on keyfile
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&keyfile_path, std::fs::Permissions::from_mode(0o600))?;
    }

    let address = signer.address();
    eprintln!("Wallet initialized successfully.");
    eprintln!("Address: {address}");
    eprintln!("Keyfile: ~/.payg/keyfile.json");
    eprintln!("Config:  ~/.payg/config.toml");
    eprintln!();
    eprintln!("Fund this address with USDC on Base to start using PAYG.");

    Ok(())
}
