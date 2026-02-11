use clap::Args;

use payg::{ConsumerConfig, PaygError, ProjectConfig};

use crate::cli::OutputFormat;

#[derive(Args)]
pub struct ChargeArgs {
    /// Amount to charge (e.g. "0.001 USDC")
    amount: Option<String>,

    /// Recipient address (overrides payg.toml)
    #[arg(long)]
    recipient: Option<String>,

    /// Validate without sending payment
    #[arg(long)]
    dry_run: bool,
}

pub async fn run(
    args: ChargeArgs,
    output: OutputFormat,
    cli_network: Option<&str>,
) -> Result<(), PaygError> {
    let consumer = ConsumerConfig::load()?;
    let network = crate::resolve_network(cli_network, &consumer)?;
    let project = ProjectConfig::load()?;

    // Resolve amount: CLI arg > payg.toml default_price
    let amount = args
        .amount
        .or_else(|| project.as_ref().and_then(|p| p.default_price.clone()))
        .ok_or_else(|| {
            PaygError::ConfigError(
                "no amount specified — pass an amount or set default_price in payg.toml"
                    .to_string(),
            )
        })?;

    // Resolve recipient: CLI arg > payg.toml recipient
    let recipient = args
        .recipient
        .or_else(|| project.as_ref().map(|p| p.recipient.clone()))
        .ok_or_else(|| {
            PaygError::ConfigError(
                "no recipient specified — pass --recipient or set recipient in payg.toml"
                    .to_string(),
            )
        })?;

    // Validate before expensive wallet load (fail fast before scrypt decryption)
    let parsed = payg::pricing::parse_price(&amount)?;
    payg::check_safety_ceiling(&parsed, &amount, &consumer)?;
    let recipient_addr: alloy_primitives::Address = recipient
        .parse()
        .map_err(|e| PaygError::ConfigError(format!("invalid recipient address: {e}")))?;

    // Load wallet once
    let signer = super::wallet_helper::load_wallet_interactive(&consumer)?;

    if args.dry_run {
        match output {
            OutputFormat::Json => {
                let result = serde_json::json!({
                    "status": "dry_run",
                    "amount": amount,
                    "recipient": recipient,
                    "wallet": format!("{}", signer.address()),
                    "network": network.name,
                    "is_testnet": network.is_testnet,
                });
                println!("{}", serde_json::to_string(&result).unwrap());
            }
            OutputFormat::Text => {
                println!("Dry run: would charge {amount} to {recipient}");
                println!("Wallet:  {}", signer.address());
                println!("Network: {} ({})", network.display_name, network.name);
            }
        }
        return Ok(());
    }

    // Use charge_raw since we already parsed and validated above
    let receipt =
        payg::charge_raw(parsed.amount, recipient_addr, &consumer, network, signer).await?;

    match output {
        OutputFormat::Json => {
            let result = serde_json::json!({
                "status": "success",
                "tx_hash": receipt.tx_hash,
                "amount": amount,
                "recipient": recipient,
                "network": network.name,
                "is_testnet": network.is_testnet,
            });
            println!("{}", serde_json::to_string(&result).unwrap());
        }
        OutputFormat::Text => {
            println!("Charged {amount} -> {recipient} (tx: {})", receipt.tx_hash);
            println!("Network: {} ({})", network.display_name, network.name);
        }
    }

    Ok(())
}
