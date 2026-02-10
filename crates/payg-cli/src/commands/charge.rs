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

pub async fn run(args: ChargeArgs, output: OutputFormat) -> Result<(), PaygError> {
    let consumer = ConsumerConfig::load()?;
    let project = ProjectConfig::load().ok();

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

    if args.dry_run {
        // Validate everything without sending
        let parsed = payg::pricing::parse_price(&amount)?;
        let ceiling = consumer.safety_ceiling_u256();

        if parsed.amount > ceiling {
            return Err(PaygError::ExceedsSafetyCeiling {
                amount: amount.clone(),
                ceiling: consumer
                    .max_charge
                    .clone()
                    .unwrap_or_else(|| "1.00 USDC".to_string()),
            });
        }

        // Verify wallet loads
        let signer = payg::wallet::load_wallet(&consumer)?;

        match output {
            OutputFormat::Json => {
                let result = serde_json::json!({
                    "status": "dry_run",
                    "amount": amount,
                    "recipient": recipient,
                    "wallet": format!("{}", signer.address()),
                });
                println!("{}", serde_json::to_string_pretty(&result).unwrap());
            }
            OutputFormat::Text => {
                eprintln!("Dry run: would charge {amount} to {recipient}");
                eprintln!("Wallet: {}", signer.address());
            }
        }
        return Ok(());
    }

    // Execute the charge
    let receipt = payg::charge(&amount, &recipient).await?;

    match output {
        OutputFormat::Json => {
            let result = serde_json::json!({
                "status": "success",
                "tx_hash": receipt.tx_hash,
                "amount": amount,
                "recipient": recipient,
            });
            println!("{}", serde_json::to_string(&result).unwrap());
        }
        OutputFormat::Text => {
            eprintln!("Charged {amount} -> {recipient} (tx: {})", receipt.tx_hash);
        }
    }

    Ok(())
}
