use alloy_primitives::U256;
use clap::Args;

use payg::{BASE_USDC_ADDRESS, ConsumerConfig, PaygError};

use crate::cli::OutputFormat;

#[derive(Args)]
pub struct BalanceArgs;

pub async fn run(_args: BalanceArgs, output: OutputFormat) -> Result<(), PaygError> {
    let config = ConsumerConfig::load()?;
    let signer = payg::wallet::load_wallet(&config)?;
    let address = signer.address();

    let rpc_url = config.rpc_url()?;

    // Query ETH balance via JSON-RPC
    let eth_balance = query_eth_balance(&rpc_url, &format!("{address}")).await?;
    let usdc_balance =
        query_erc20_balance(&rpc_url, BASE_USDC_ADDRESS, &format!("{address}")).await?;

    let eth_formatted = format_wei_as_eth(eth_balance);
    let usdc_formatted = format_token_amount(usdc_balance, 6);

    match output {
        OutputFormat::Json => {
            let result = serde_json::json!({
                "address": format!("{address}"),
                "eth": eth_formatted,
                "usdc": usdc_formatted,
                "chain": "base",
            });
            println!("{}", serde_json::to_string(&result).unwrap());
        }
        OutputFormat::Text => {
            eprintln!("Wallet {address}:");
            eprintln!("  ETH:  {eth_formatted}");
            eprintln!("  USDC: {usdc_formatted}");
            eprintln!("  Chain: Base");
        }
    }

    Ok(())
}

/// Query ETH balance via eth_getBalance JSON-RPC.
async fn query_eth_balance(rpc_url: &str, address: &str) -> Result<U256, PaygError> {
    let client = reqwest::Client::new();
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
        .ok_or_else(|| PaygError::PaymentFailed("invalid eth_getBalance response".to_string()))?;

    U256::from_str_radix(hex.trim_start_matches("0x"), 16)
        .map_err(|e| PaygError::PaymentFailed(format!("bad hex: {e}")))
}

/// Query ERC-20 balance via eth_call to balanceOf(address).
async fn query_erc20_balance(
    rpc_url: &str,
    token_address: &str,
    wallet_address: &str,
) -> Result<U256, PaygError> {
    // balanceOf(address) selector = 0x70a08231
    let wallet_padded = format!("{:0>64}", wallet_address.trim_start_matches("0x"));
    let data = format!("0x70a08231{wallet_padded}");

    let client = reqwest::Client::new();
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
        .ok_or_else(|| PaygError::PaymentFailed("invalid eth_call response".to_string()))?;

    U256::from_str_radix(hex.trim_start_matches("0x"), 16)
        .map_err(|e| PaygError::PaymentFailed(format!("bad hex: {e}")))
}

fn format_wei_as_eth(wei: U256) -> String {
    let divisor = U256::from(10u64).pow(U256::from(18u64));
    if wei.is_zero() {
        return "0.0".to_string();
    }
    let whole = wei / divisor;
    let frac = wei % divisor;
    // Show up to 6 decimal places
    let frac_str = format!("{:0>18}", frac);
    let trimmed = frac_str[..6].trim_end_matches('0');
    if trimmed.is_empty() {
        format!("{whole}.0")
    } else {
        format!("{whole}.{trimmed}")
    }
}

fn format_token_amount(amount: U256, decimals: u32) -> String {
    let divisor = U256::from(10u64).pow(U256::from(decimals));
    if amount.is_zero() {
        return "0.0".to_string();
    }
    let whole = amount / divisor;
    let frac = amount % divisor;
    let frac_str = format!("{:0>width$}", frac, width = decimals as usize);
    let trimmed = frac_str.trim_end_matches('0');
    if trimmed.is_empty() {
        format!("{whole}.0")
    } else {
        format!("{whole}.{trimmed}")
    }
}
