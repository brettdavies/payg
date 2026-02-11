use std::path::PathBuf;

use alloy_signer_local::PrivateKeySigner;
use clap::Args;

use payg::PaygError;

use crate::cli::OutputFormat;

#[derive(Args)]
pub struct InitArgs {
    /// Run without interactive prompts (uses env vars)
    #[arg(long)]
    non_interactive: bool,
}

pub async fn run(
    args: InitArgs,
    output: OutputFormat,
    cli_network: Option<&str>,
) -> Result<(), PaygError> {
    // Resolve network early so we can write it to config and show correct messaging.
    // For init, we don't have a ConsumerConfig yet. Clap handles CLI flag + PAYG_NETWORK
    // env var, so cli_network is Some if either was provided. Otherwise use default.
    let network = match cli_network {
        Some(name) => payg::network::resolve_network_config(name)?,
        None => payg::network::DEFAULT_NETWORK,
    };
    if !network.is_testnet {
        eprintln!("WARNING: operating on Base MAINNET — real funds will be used");
    }

    let home = payg::config::home_dir()?;

    let payg_dir = home.join(".payg");
    create_dir_secure(&payg_dir)?;

    let keyfile_path = payg_dir.join("keyfile.json");

    // Atomic existence check: create_new fails if file already exists (no TOCTOU race)
    #[cfg(unix)]
    {
        use std::fs::OpenOptions;
        use std::os::unix::fs::OpenOptionsExt;
        OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&keyfile_path)
            .map_err(|e| {
                if e.kind() == std::io::ErrorKind::AlreadyExists {
                    PaygError::ConfigError(
                        "wallet already exists at ~/.payg/keyfile.json — delete it first to reinitialize"
                            .to_string(),
                    )
                } else {
                    PaygError::Io(e)
                }
            })?;
    }
    #[cfg(not(unix))]
    {
        use std::fs::OpenOptions;
        OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&keyfile_path)
            .map_err(|e| {
                if e.kind() == std::io::ErrorKind::AlreadyExists {
                    PaygError::ConfigError(
                        "wallet already exists at ~/.payg/keyfile.json — delete it first to reinitialize"
                            .to_string(),
                    )
                } else {
                    PaygError::Io(e)
                }
            })?;
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

    // Generate new key and encrypt to keyfile (overwrites the placeholder we created)
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

    // Ensure keyfile has correct permissions after encrypt_keystore overwrites it
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&keyfile_path, std::fs::Permissions::from_mode(0o600))?;
    }

    // Write default config with explicit network
    let config_path = payg_dir.join("config.toml");
    if !config_path.exists() {
        let config = format!(
            "keyfile = \"~/.payg/keyfile.json\"\nmax_charge = \"{}\"\nnetwork = \"{}\"\n",
            payg::DEFAULT_SAFETY_CEILING_DISPLAY,
            network.name,
        );
        std::fs::write(&config_path, config)?;
    }

    let address = signer.address();

    match output {
        OutputFormat::Json => {
            let faucet_url = if network.is_testnet {
                Some("https://faucet.circle.com/")
            } else {
                None
            };
            let result = serde_json::json!({
                "status": "created",
                "address": format!("{address}"),
                "keyfile": "~/.payg/keyfile.json",
                "config": "~/.payg/config.toml",
                "network": network.name,
                "faucet_url": faucet_url,
            });
            println!("{}", serde_json::to_string(&result).unwrap());
        }
        OutputFormat::Text => {
            println!("Wallet initialized successfully.");
            println!("Address: {address}");
            println!("Keyfile: ~/.payg/keyfile.json");
            println!("Config:  ~/.payg/config.toml");
            println!("Network: {} ({})", network.display_name, network.name);
            println!();
            if network.is_testnet {
                println!("Get testnet USDC from https://faucet.circle.com/");
            } else {
                println!("Fund this address with USDC on Base to start using PAYG.");
            }
        }
    }

    Ok(())
}

/// Create a directory with 0o700 permissions on Unix.
fn create_dir_secure(path: &PathBuf) -> Result<(), PaygError> {
    if path.exists() {
        return Ok(());
    }

    #[cfg(unix)]
    {
        use std::fs;
        use std::os::unix::fs::DirBuilderExt;
        fs::DirBuilder::new()
            .recursive(true)
            .mode(0o700)
            .create(path)?;
    }

    #[cfg(not(unix))]
    {
        std::fs::create_dir_all(path)?;
    }

    Ok(())
}
