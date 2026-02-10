use std::time::Duration;

use alloy_primitives::U256;
use clap::Args;

use payg::{BASE_USDC_ADDRESS, ConsumerConfig, PaygError};

use crate::cli::OutputFormat;

#[derive(Args)]
pub struct BalanceArgs;

pub async fn run(_args: BalanceArgs, output: OutputFormat) -> Result<(), PaygError> {
    let config = ConsumerConfig::load()?;
    let signer = super::wallet_helper::load_wallet_interactive(&config)?;
    let address = signer.address();

    let rpc_url = config.rpc_url()?;
    let address_str = format!("{address}");

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .connect_timeout(Duration::from_secs(10))
        .build()
        .map_err(|e| PaygError::Http(e.to_string()))?;

    // Query ETH and USDC balances in parallel
    let (eth_balance, usdc_balance) = tokio::try_join!(
        query_eth_balance(&client, &rpc_url, &address_str),
        query_erc20_balance(&client, &rpc_url, BASE_USDC_ADDRESS, &address_str),
    )?;

    let eth_formatted = format_token_amount(eth_balance, 18);
    let usdc_formatted = format_token_amount(usdc_balance, 6);

    match output {
        OutputFormat::Json => {
            let result = serde_json::json!({
                "address": address_str,
                "eth": eth_formatted,
                "eth_wei": eth_balance.to_string(),
                "usdc": usdc_formatted,
                "usdc_raw": usdc_balance.to_string(),
                "chain": "base",
            });
            println!("{}", serde_json::to_string(&result).unwrap());
        }
        OutputFormat::Text => {
            println!("Wallet {address}:");
            println!("  ETH:  {eth_formatted}");
            println!("  USDC: {usdc_formatted}");
            println!("  Chain: Base");
        }
    }

    Ok(())
}

/// Query ETH balance via eth_getBalance JSON-RPC.
async fn query_eth_balance(
    client: &reqwest::Client,
    rpc_url: &str,
    address: &str,
) -> Result<U256, PaygError> {
    let body = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "eth_getBalance",
        "params": [address, "latest"],
        "id": 1
    });

    let response = client
        .post(rpc_url)
        .json(&body)
        .send()
        .await
        .map_err(|e| PaygError::Http(e.to_string()))?;

    let resp: serde_json::Value = response
        .json()
        .await
        .map_err(|e| PaygError::Http(e.to_string()))?;

    let hex = resp["result"]
        .as_str()
        .ok_or_else(|| PaygError::Http("invalid eth_getBalance response".to_string()))?;

    U256::from_str_radix(hex.trim_start_matches("0x"), 16)
        .map_err(|e| PaygError::Http(format!("bad hex: {e}")))
}

/// Query ERC-20 balance via eth_call to balanceOf(address).
async fn query_erc20_balance(
    client: &reqwest::Client,
    rpc_url: &str,
    token_address: &str,
    wallet_address: &str,
) -> Result<U256, PaygError> {
    // balanceOf(address) selector = 0x70a08231
    let wallet_padded = format!("{:0>64}", wallet_address.trim_start_matches("0x"));
    let data = format!("0x70a08231{wallet_padded}");

    let body = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "eth_call",
        "params": [{
            "to": token_address,
            "data": data
        }, "latest"],
        "id": 1
    });

    let response = client
        .post(rpc_url)
        .json(&body)
        .send()
        .await
        .map_err(|e| PaygError::Http(e.to_string()))?;

    let resp: serde_json::Value = response
        .json()
        .await
        .map_err(|e| PaygError::Http(e.to_string()))?;

    let hex = resp["result"]
        .as_str()
        .ok_or_else(|| PaygError::Http("invalid eth_call response".to_string()))?;

    U256::from_str_radix(hex.trim_start_matches("0x"), 16)
        .map_err(|e| PaygError::Http(format!("bad hex: {e}")))
}

fn format_token_amount(amount: U256, decimals: u32) -> String {
    let divisor = U256::from(10u64).pow(U256::from(decimals));
    if amount.is_zero() {
        return "0.0".to_string();
    }
    let whole = amount / divisor;
    let frac = amount % divisor;
    let frac_str = format!("{:0>width$}", frac, width = decimals as usize);
    // For display, truncate to 6 significant decimal places
    let display_len = frac_str.len().min(6);
    let trimmed = frac_str[..display_len].trim_end_matches('0');
    if trimmed.is_empty() {
        format!("{whole}.0")
    } else {
        format!("{whole}.{trimmed}")
    }
}
