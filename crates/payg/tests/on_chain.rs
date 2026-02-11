//! On-chain verification tests for NetworkConfig presets.
//!
//! These tests make read-only RPC calls to verify that hardcoded values
//! in NetworkConfig match deployed contract reality. Run with:
//!
//!   cargo test -- --ignored
//!
//! Filter by network:
//!   cargo test sepolia -- --ignored
//!   cargo test mainnet -- --ignored
//!
//! Run e2e charge test (requires PAYG_PRIVATE_KEY with funded testnet wallet):
//!   cargo test e2e -- --ignored

use std::sync::LazyLock;
use std::time::Duration;

use payg::network::{BASE_MAINNET, BASE_SEPOLIA, NetworkConfig};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Shared HTTP client for all tests. Eliminates redundant TLS handshakes
/// across 12+ parallel tests hitting 2 RPC endpoints.
static CLIENT: LazyLock<reqwest::Client> = LazyLock::new(|| {
    reqwest::Client::builder()
        .timeout(Duration::from_secs(15))
        .connect_timeout(Duration::from_secs(5))
        .build()
        .expect("failed to build HTTP client")
});

fn rpc_url_for(network: &NetworkConfig) -> String {
    std::env::var("PAYG_TEST_RPC_URL").unwrap_or_else(|_| network.default_rpc_url.to_string())
}

/// Send a JSON-RPC request and return the `result` field.
async fn rpc_call(
    client: &reqwest::Client,
    rpc_url: &str,
    method: &str,
    params: serde_json::Value,
) -> serde_json::Value {
    let body = serde_json::json!({
        "jsonrpc": "2.0",
        "method": method,
        "params": params,
        "id": 1
    });

    let resp: serde_json::Value = client
        .post(rpc_url)
        .json(&body)
        .send()
        .await
        .unwrap_or_else(|e| panic!("RPC request to {rpc_url} failed: {e}"))
        .json()
        .await
        .unwrap_or_else(|e| panic!("RPC response from {rpc_url} not JSON: {e}"));

    if let Some(err) = resp.get("error") {
        panic!("RPC error from {rpc_url}: {err}");
    }

    resp["result"].clone()
}

/// Call a view function on a contract and return the raw hex result (without 0x prefix).
async fn eth_call_hex(client: &reqwest::Client, rpc_url: &str, to: &str, selector: &str) -> String {
    let result = rpc_call(
        client,
        rpc_url,
        "eth_call",
        serde_json::json!([{"to": to, "data": selector}, "latest"]),
    )
    .await;

    result
        .as_str()
        .unwrap_or_else(|| panic!("eth_call to {to} returned non-string: {result}"))
        .trim_start_matches("0x")
        .to_string()
}

/// Decode an ABI-encoded string (offset + length + data).
fn decode_abi_string(hex: &str) -> String {
    // ABI layout: 32 bytes offset + 32 bytes length + N bytes UTF-8 data
    // Offset is always 0x20 (32) for a single string return.
    // Length starts at byte 64 (hex chars 128).
    if hex.len() < 128 {
        panic!("ABI string too short to decode: 0x{hex}");
    }

    let len_hex = &hex[64..128];
    let len = usize::from_str_radix(len_hex, 16)
        .unwrap_or_else(|e| panic!("bad ABI string length '0x{len_hex}': {e}"));

    // Data starts at byte 96 (hex char 128), take `len` bytes (len*2 hex chars)
    let data_hex = &hex[128..128 + len * 2];
    let bytes: Vec<u8> = (0..data_hex.len())
        .step_by(2)
        .map(|i| {
            u8::from_str_radix(&data_hex[i..i + 2], 16)
                .unwrap_or_else(|e| panic!("bad hex byte at {i}: {e}"))
        })
        .collect();

    String::from_utf8(bytes).unwrap_or_else(|e| panic!("ABI string not UTF-8: {e}"))
}

/// Decode an ABI-encoded uint8 (32-byte padded).
fn decode_abi_uint8(hex: &str) -> u8 {
    // Last byte of a 32-byte (64 hex char) word
    if hex.len() < 64 {
        panic!("ABI uint8 too short: 0x{hex}");
    }
    let byte_hex = &hex[62..64];
    u8::from_str_radix(byte_hex, 16).unwrap_or_else(|e| panic!("bad uint8 hex '0x{byte_hex}': {e}"))
}

// ---------------------------------------------------------------------------
// Parameterised verify helpers (one per assertion, shared by both networks)
// ---------------------------------------------------------------------------

async fn verify_chain_id(network: &payg::network::NetworkConfig) {
    let rpc_url = rpc_url_for(network);
    let result = rpc_call(&CLIENT, &rpc_url, "eth_chainId", serde_json::json!([])).await;
    let hex = result.as_str().expect("eth_chainId returned non-string");
    let chain_id = u64::from_str_radix(hex.trim_start_matches("0x"), 16).expect("bad chain_id hex");
    assert_eq!(
        chain_id, network.chain_id,
        "chain_id mismatch for {}",
        network.name
    );
}

async fn verify_usdc_contract_exists(network: &payg::network::NetworkConfig) {
    let rpc_url = rpc_url_for(network);
    let result = rpc_call(
        &CLIENT,
        &rpc_url,
        "eth_getCode",
        serde_json::json!([network.usdc_address, "latest"]),
    )
    .await;
    let code = result.as_str().expect("eth_getCode returned non-string");
    assert!(
        code.len() > 4,
        "no contract at USDC address {} on {}",
        network.usdc_address,
        network.name
    );
}

async fn verify_usdc_decimals(network: &payg::network::NetworkConfig) {
    let rpc_url = rpc_url_for(network);
    let hex = eth_call_hex(&CLIENT, &rpc_url, network.usdc_address, "0x313ce567").await;
    assert_eq!(
        decode_abi_uint8(&hex),
        6,
        "USDC decimals should be 6 on {}",
        network.name
    );
}

async fn verify_usdc_name(network: &payg::network::NetworkConfig) {
    let rpc_url = rpc_url_for(network);
    let hex = eth_call_hex(&CLIENT, &rpc_url, network.usdc_address, "0x06fdde03").await;
    let name = decode_abi_string(&hex);
    assert_eq!(
        name, network.eip712_name,
        "USDC name() should be '{}' on {}, got '{name}'",
        network.eip712_name, network.name
    );
}

async fn verify_usdc_version(network: &payg::network::NetworkConfig) {
    let rpc_url = rpc_url_for(network);
    let hex = eth_call_hex(&CLIENT, &rpc_url, network.usdc_address, "0x54fd4d50").await;
    let version = decode_abi_string(&hex);
    assert_eq!(
        version, network.eip712_version,
        "USDC version() should be '{}' on {}, got '{version}'",
        network.eip712_version, network.name
    );
}

async fn verify_rpc_reachable(network: &payg::network::NetworkConfig) {
    let rpc_url = rpc_url_for(network);
    let result = rpc_call(&CLIENT, &rpc_url, "eth_blockNumber", serde_json::json!([])).await;
    let hex = result
        .as_str()
        .expect("eth_blockNumber returned non-string");
    let block =
        u64::from_str_radix(hex.trim_start_matches("0x"), 16).expect("bad block number hex");
    assert!(
        block > 0,
        "{} block number should be > 0, got {block}",
        network.name
    );
}

// ---------------------------------------------------------------------------
// Base Sepolia Tests
// ---------------------------------------------------------------------------

#[tokio::test]
#[ignore]
async fn sepolia_chain_id_matches() {
    verify_chain_id(&BASE_SEPOLIA).await;
}

#[tokio::test]
#[ignore]
async fn sepolia_usdc_contract_exists() {
    verify_usdc_contract_exists(&BASE_SEPOLIA).await;
}

#[tokio::test]
#[ignore]
async fn sepolia_usdc_decimals_is_6() {
    verify_usdc_decimals(&BASE_SEPOLIA).await;
}

#[tokio::test]
#[ignore]
async fn sepolia_usdc_name_matches_eip712() {
    verify_usdc_name(&BASE_SEPOLIA).await;
}

#[tokio::test]
#[ignore]
async fn sepolia_usdc_version_is_2() {
    verify_usdc_version(&BASE_SEPOLIA).await;
}

#[tokio::test]
#[ignore]
async fn sepolia_rpc_is_reachable() {
    verify_rpc_reachable(&BASE_SEPOLIA).await;
}

// ---------------------------------------------------------------------------
// Base Mainnet Tests
// ---------------------------------------------------------------------------

#[tokio::test]
#[ignore]
async fn mainnet_chain_id_matches() {
    verify_chain_id(&BASE_MAINNET).await;
}

#[tokio::test]
#[ignore]
async fn mainnet_usdc_contract_exists() {
    verify_usdc_contract_exists(&BASE_MAINNET).await;
}

#[tokio::test]
#[ignore]
async fn mainnet_usdc_decimals_is_6() {
    verify_usdc_decimals(&BASE_MAINNET).await;
}

#[tokio::test]
#[ignore]
async fn mainnet_usdc_name_matches_eip712() {
    verify_usdc_name(&BASE_MAINNET).await;
}

#[tokio::test]
#[ignore]
async fn mainnet_usdc_version_is_2() {
    verify_usdc_version(&BASE_MAINNET).await;
}

#[tokio::test]
#[ignore]
async fn mainnet_rpc_is_reachable() {
    verify_rpc_reachable(&BASE_MAINNET).await;
}

// ---------------------------------------------------------------------------
// End-to-end charge on Base Sepolia (requires funded wallet)
// ---------------------------------------------------------------------------

/// Maximum charge amount for e2e tests (in USDC base units, 6 decimals).
/// 0.01 USDC absolute ceiling. DO NOT increase without security review.
const E2E_MAX_CHARGE_USDC: u64 = 10_000;

#[tokio::test]
#[ignore]
async fn e2e_charge_sepolia() {
    // Gate 1: Wallet — skip if PAYG_PRIVATE_KEY not set
    let Ok(_key) = std::env::var("PAYG_PRIVATE_KEY") else {
        eprintln!("Skipping e2e_charge_sepolia: PAYG_PRIVATE_KEY not set");
        eprintln!("To run: export PAYG_PRIVATE_KEY=<base-sepolia-funded-key>");
        eprintln!("Get testnet USDC from https://faucet.circle.com/");
        eprintln!("(no ETH needed — x402 facilitator pays gas)");
        return;
    };

    let network = &BASE_SEPOLIA;
    assert!(
        network.is_testnet,
        "e2e charge tests MUST run on testnet only"
    );

    // Gate 2: Facilitator health check — skip if unreachable
    let facilitator_url = payg::DEFAULT_FACILITATOR_URL;
    let health_client = reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .expect("failed to build health check client");
    if health_client.get(facilitator_url).send().await.is_err() {
        eprintln!(
            "Skipping e2e_charge_sepolia: x402 facilitator at {facilitator_url} is unreachable"
        );
        return;
    }

    // Load wallet and config using the library's own machinery
    let config = payg::ConsumerConfig::default();
    let password: Option<&str> = None;
    let signer = payg::wallet::load_wallet(&config, password)
        .expect("failed to load wallet from PAYG_PRIVATE_KEY");

    let recipient = signer.address(); // charge to self to avoid losing testnet USDC

    // Gate 3: USDC balance — skip if insufficient
    let rpc_url = rpc_url_for(network);
    let wallet_padded = format!("{:0>64}", format!("{:x}", recipient));
    let data = format!("0x70a08231{wallet_padded}");
    let balance_result = rpc_call(
        &CLIENT,
        &rpc_url,
        "eth_call",
        serde_json::json!([{"to": network.usdc_address, "data": data}, "latest"]),
    )
    .await;
    let balance_hex = balance_result
        .as_str()
        .expect("balanceOf returned non-string")
        .trim_start_matches("0x");
    let balance = u64::from_str_radix(balance_hex.trim_start_matches('0'), 16).unwrap_or(0);

    // 0.001 USDC = 1000 units (6 decimals)
    let charge_amount = 1000u64;
    assert!(
        charge_amount <= E2E_MAX_CHARGE_USDC,
        "test charge {charge_amount} exceeds test ceiling {E2E_MAX_CHARGE_USDC}"
    );

    if balance < charge_amount {
        eprintln!(
            "Skipping e2e_charge_sepolia: insufficient USDC balance ({balance} < {charge_amount})"
        );
        eprintln!("Get testnet USDC from https://faucet.circle.com/");
        eprintln!("(no ETH needed — x402 facilitator pays gas)");
        return;
    }

    // Execute the charge
    let amount = alloy_primitives::U256::from(charge_amount);
    let receipt = payg::charge_raw(amount, recipient, &config, network, signer)
        .await
        .expect("charge_raw failed on base-sepolia");

    // Verify receipt format
    assert!(
        receipt.tx_hash.starts_with("0x"),
        "tx_hash should start with 0x, got: {}",
        receipt.tx_hash
    );
    assert_eq!(
        receipt.tx_hash.len(),
        66,
        "tx_hash should be 66 chars (0x + 32 bytes hex), got {} chars: {}",
        receipt.tx_hash.len(),
        receipt.tx_hash
    );

    // Verify on-chain confirmation
    let tx_receipt = rpc_call(
        &CLIENT,
        &rpc_url,
        "eth_getTransactionReceipt",
        serde_json::json!([&receipt.tx_hash]),
    )
    .await;
    let status = tx_receipt["status"]
        .as_str()
        .expect("transaction receipt missing status field");
    assert_eq!(status, "0x1", "on-chain tx did not succeed");

    eprintln!("e2e charge succeeded: tx_hash={}", receipt.tx_hash);
    eprintln!(
        "View on explorer: https://sepolia.basescan.org/tx/{}",
        receipt.tx_hash
    );
}
