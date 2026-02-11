use clap::Args;

use payg::{ConsumerConfig, PaygError};

use crate::cli::OutputFormat;

#[derive(Args)]
pub struct AddressArgs;

pub async fn run(
    _args: AddressArgs,
    output: OutputFormat,
    cli_network: Option<&str>,
) -> Result<(), PaygError> {
    let config = ConsumerConfig::load()?;
    let network = crate::resolve_network(cli_network, &config)?;
    let signer = super::wallet_helper::load_wallet_interactive(&config)?;
    let address = signer.address();

    match output {
        OutputFormat::Json => {
            let result = serde_json::json!({
                "status": "ok",
                "address": format!("{address}"),
                "network": network.name,
                "is_testnet": network.is_testnet,
            });
            println!("{}", serde_json::to_string(&result).unwrap());
        }
        OutputFormat::Text => {
            println!("{address}");
            println!("Network: {} ({})", network.display_name, network.name);
        }
    }

    Ok(())
}
