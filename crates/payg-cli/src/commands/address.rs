use clap::Args;

use payg::{ConsumerConfig, PaygError};

use crate::cli::OutputFormat;

#[derive(Args)]
pub struct AddressArgs;

pub async fn run(_args: AddressArgs, output: OutputFormat) -> Result<(), PaygError> {
    let config = ConsumerConfig::load()?;
    let signer = payg::wallet::load_wallet(&config)?;
    let address = signer.address();

    match output {
        OutputFormat::Json => {
            let result = serde_json::json!({
                "address": format!("{address}"),
            });
            println!("{}", serde_json::to_string(&result).unwrap());
        }
        OutputFormat::Text => {
            println!("{address}");
        }
    }

    Ok(())
}
