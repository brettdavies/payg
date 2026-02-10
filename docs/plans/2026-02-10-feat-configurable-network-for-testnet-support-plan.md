---
title: Configurable Network Selection for Testnet Support
type: feat
date: 2026-02-10
---

# Configurable Network Selection for Testnet Support

## Overview

Make PAYG network-aware so it can target Base mainnet or Base Sepolia testnet. Currently, chain ID, USDC address, EIP-712 domain parameters, x402 network name, and default RPC URL are all hardcoded to Base mainnet. This blocks end-to-end testing without spending real money.

The design bundles all 6 network-coupled values into a `NetworkConfig` struct with two presets (`base` and `base-sepolia`). Default is `base-sepolia` to prevent accidental mainnet charges.

## Problem Statement

- Cannot test the full payment flow without spending real USDC on Base mainnet
- 14 code locations across 5 files have hardcoded mainnet values
- The EIP-712 domain name differs between mainnet (`"USD Coin"`) and Sepolia (`"USDC"`) — using the wrong name produces invalid signatures that the facilitator silently rejects

## Proposed Solution

### NetworkConfig struct

Bundle the 6 tightly coupled values into a single struct with two compile-time presets:

```rust
pub struct NetworkConfig {
    pub chain_id: u64,
    pub usdc_address: &'static str,
    pub eip712_name: &'static str,
    pub eip712_version: &'static str,
    pub network_name: &'static str,      // x402 protocol field
    pub default_rpc_url: &'static str,
}
```

| Field | `base` (mainnet) | `base-sepolia` (testnet) |
|-------|-------------------|--------------------------|
| `chain_id` | `8453` | `84532` |
| `usdc_address` | `0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913` | `0x036CbD53842c5426634e7929541eC2318f3dCF7e` |
| `eip712_name` | `"USD Coin"` | `"USDC"` |
| `eip712_version` | `"2"` | `"2"` |
| `network_name` | `"base"` | `"base-sepolia"` |
| `default_rpc_url` | `https://mainnet.base.org` | `https://sepolia.base.org` |

Sources: x402-rs `networks.rs` (lines 148-170), Circle USDC testnet deployment docs.

### Selection precedence

```
CLI --network flag > PAYG_NETWORK env var > config.toml network field > default (base-sepolia)
```

Only two valid values: `"base"` and `"base-sepolia"`. Invalid values produce an error listing the valid options.

### Default: `base-sepolia` (safety-first)

Users must explicitly opt into mainnet. This prevents accidental real-money charges during development and testing. `payg init` writes the resolved network to `~/.payg/config.toml` so future default changes don't silently affect existing users.

## Technical Considerations

### Architecture: Threading NetworkConfig through the stack

`NetworkConfig` is resolved once at the top of the call chain and threaded down:

1. `ConsumerConfig` gains an `Option<String>` field: `network`
2. A new function `resolve_network()` checks CLI arg > env var > config field > default
3. `Backend::from_config()` gains a `&NetworkConfig` parameter
4. `X402Backend::new()` stores the network config fields it needs
5. `charge_with_config()` gains a `&NetworkConfig` parameter
6. Convenience `charge()` resolves the network internally

The `NetworkConfig` presets are `const` — no heap allocation, no parsing at runtime for the two known networks.

### x402 backend changes (the bulk of the work)

The x402 backend currently imports `BASE_CHAIN_ID` and `BASE_USDC_ADDRESS` as constants and hardcodes `"base"`, `"USD Coin"`, and `"2"` as string literals. All 7 locations must read from `NetworkConfig` instead:

| Location | Current | After |
|----------|---------|-------|
| `x402.rs:50` `chain_id` in `Eip3009SigningParams` | `BASE_CHAIN_ID` | `self.network.chain_id` |
| `x402.rs:51` `asset_address` | `self.usdc_address` (from const) | `self.network.usdc_address` (parsed) |
| `x402.rs:56` EIP-712 `name` | `"USD Coin"` | `self.network.eip712_name` |
| `x402.rs:57` EIP-712 `version` | `"2"` | `self.network.eip712_version` |
| `x402.rs:138` payload `network` | `"base"` | `self.network.network_name` |
| `x402.rs:143` requirements `network` | `"base"` | `self.network.network_name` |
| `x402.rs:152-153` extra `name`/`version` | `"USD Coin"` / `"2"` | `self.network.eip712_name`/`version` |

### ETH backend: no changes needed

The ETH backend gets chain_id from the RPC provider (alloy auto-fills) and doesn't use USDC address or EIP-712 parameters. It only needs `rpc_url()` which already has env var override support. The `NetworkConfig` default RPC URL feeds into `ConsumerConfig::rpc_url()` as the new default instead of the hardcoded `"https://mainnet.base.org"`.

### Balance command changes

`balance.rs` imports `BASE_USDC_ADDRESS` for the `balanceOf` query and hardcodes `"base"` / `"Base"` in output. It needs to receive `NetworkConfig` and use:
- `network.usdc_address` for the ERC-20 query
- `network.network_name` / display name for output

### Config changes

`config.rs` changes:
- `ConsumerConfig` gains `network: Option<String>` field (deserialized from TOML)
- `rpc_url()` default changes from hardcoded mainnet URL to `network.default_rpc_url`
- New `resolve_network()` function: env var > config field > `"base-sepolia"`

### Init command changes

- Write `network = "base-sepolia"` (or resolved value) to `~/.payg/config.toml`
- Change messaging: testnet says "Get testnet USDC from https://faucet.circle.com/", mainnet says "Fund this address with USDC on Base"

### CLI changes

Add `--network` as a global flag on `Cli` struct:

```rust
#[arg(long, global = true)]
pub network: Option<String>,
```

Pass through to all commands that need it.

### Library API changes

`charge()` resolves network internally (from env var / config). `charge_with_config()` gains a `&NetworkConfig` parameter for callers who need explicit control. This is a breaking change to the function signature, acceptable at v0.1.0.

### Facilitator URL

The x402.org facilitator supports both Base mainnet and Base Sepolia — confirmed from the x402-rs codebase (the facilitator routes based on the `network` field in the payload). Same default URL works for both networks. No 7th field needed in `NetworkConfig`.

### Dry-run output

`payg charge --dry-run` should display the resolved network so users can verify before committing.

## Acceptance Criteria

- [ ] `NetworkConfig` struct with `base` and `base-sepolia` presets
- [ ] `PAYG_NETWORK` env var selects network
- [ ] `--network` CLI flag selects network (highest priority)
- [ ] `network` field in `~/.payg/config.toml` selects network
- [ ] Default is `base-sepolia` when nothing is set
- [ ] `payg init` writes explicit `network` to config
- [ ] `payg charge` signs with correct chain_id and EIP-712 domain for selected network
- [ ] `payg charge --dry-run` displays resolved network
- [ ] `payg balance` queries correct USDC contract for selected network
- [ ] `payg balance` displays correct chain name
- [ ] Invalid `PAYG_NETWORK` values produce clear error with valid options
- [ ] All 6 existing unit tests still pass
- [ ] `cargo clippy` clean on both feature combinations
- [ ] README updated with network selection docs

## Success Metrics

End-to-end testnet charge succeeds: `payg charge "0.001 USDC" --recipient <addr>` on Base Sepolia with test USDC from Circle faucet.

## Dependencies & Risks

**Risk: EIP-712 domain mismatch.** The Sepolia USDC contract uses `"USDC"` not `"USD Coin"` as its EIP-712 name. Confirmed from x402-rs `networks.rs` lines 160-170. Using the wrong name produces signatures the facilitator cannot verify — charges fail silently with an opaque "invalid signature" error.

**Risk: Facilitator rejects testnet.** If x402.org/facilitator doesn't support `base-sepolia`, all default charges fail. Mitigation: test against the facilitator before merging. Fallback: use `https://facilitator.x402.rs` which is confirmed to support testnet.

**Risk: v0.1 users.** Default flips from mainnet to testnet. Since we're pre-1.0 and have no known production users, this is acceptable. `payg init` writing the network explicitly prevents future default changes from affecting existing users.

## MVP

### `crates/payg/src/network.rs` (new file)

```rust
/// Network-specific configuration for Base L2 chains.
#[derive(Debug, Clone, Copy)]
pub struct NetworkConfig {
    pub chain_id: u64,
    pub usdc_address: &'static str,
    pub eip712_name: &'static str,
    pub eip712_version: &'static str,
    pub network_name: &'static str,
    pub default_rpc_url: &'static str,
}

pub const BASE_MAINNET: NetworkConfig = NetworkConfig {
    chain_id: 8453,
    usdc_address: "0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913",
    eip712_name: "USD Coin",
    eip712_version: "2",
    network_name: "base",
    default_rpc_url: "https://mainnet.base.org",
};

pub const BASE_SEPOLIA: NetworkConfig = NetworkConfig {
    chain_id: 84532,
    usdc_address: "0x036CbD53842c5426634e7929541eC2318f3dCF7e",
    eip712_name: "USDC",
    eip712_version: "2",
    network_name: "base-sepolia",
    default_rpc_url: "https://sepolia.base.org",
};

pub const DEFAULT_NETWORK: &NetworkConfig = &BASE_SEPOLIA;

/// Resolve a network name to its config. Returns error for unknown networks.
pub fn resolve_network_config(name: &str) -> Result<&'static NetworkConfig, PaygError> {
    match name {
        "base" => Ok(&BASE_MAINNET),
        "base-sepolia" => Ok(&BASE_SEPOLIA),
        _ => Err(PaygError::ConfigError(format!(
            "unknown network '{name}' — valid options: base, base-sepolia"
        ))),
    }
}
```

### Changes to `crates/payg/src/config.rs`

```rust
// Add to ConsumerConfig:
pub network: Option<String>,

// New method:
pub fn resolve_network(&self) -> Result<&'static NetworkConfig, PaygError> {
    let name = std::env::var("PAYG_NETWORK").ok()
        .or_else(|| self.network.clone())
        .unwrap_or_else(|| "base-sepolia".to_string());
    resolve_network_config(&name)
}

// Update rpc_url() to use network default:
pub fn rpc_url(&self, network: &NetworkConfig) -> Result<String, PaygError> {
    let url = std::env::var("PAYG_RPC_URL").unwrap_or_else(|_| {
        self.rpc_url
            .clone()
            .unwrap_or_else(|| network.default_rpc_url.to_string())
    });
    validate_url(&url, "rpc_url")?;
    Ok(url)
}
```

### Changes to `crates/payg/src/lib.rs`

```rust
// Remove old constants:
// pub const BASE_CHAIN_ID: u64 = 8453;
// pub const BASE_USDC_ADDRESS: &str = "...";

// Add module:
pub mod network;
pub use network::NetworkConfig;

// Update charge():
pub async fn charge(amount: &str, recipient: &str) -> Result<ChargeReceipt, PaygError> {
    let consumer_config = ConsumerConfig::load()?;
    let network = consumer_config.resolve_network()?;
    let password = std::env::var("PAYG_KEY_PASSWORD").ok();
    let signer = wallet::load_wallet(&consumer_config, password.as_deref())?;
    charge_with_config(amount, recipient, &consumer_config, network, signer).await
}

// Update charge_with_config():
pub async fn charge_with_config(
    amount: &str,
    recipient: &str,
    consumer_config: &ConsumerConfig,
    network: &NetworkConfig,
    signer: PrivateKeySigner,
) -> Result<ChargeReceipt, PaygError> {
    // ... same logic, pass network to Backend::from_config()
}
```

### Changes to `crates/payg/src/backend/x402.rs`

```rust
pub struct X402Backend {
    facilitator_url: String,
    signer: PrivateKeySigner,
    usdc_address: Address,
    client: reqwest::Client,
    network: &'static NetworkConfig,  // NEW
}

// In charge(), use self.network.chain_id, self.network.eip712_name, etc.
// In build_settle_request(), use self.network.network_name
```

### Changes to `crates/payg-cli/src/cli.rs`

```rust
#[command(...)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,

    #[arg(long, global = true, default_value = "text")]
    pub output: OutputFormat,

    /// Network: "base" (mainnet) or "base-sepolia" (testnet)
    #[arg(long, global = true)]
    pub network: Option<String>,
}
```

### Changes to CLI commands

All commands that need network config (`charge`, `balance`, `init`) resolve it with:

```rust
// CLI --network flag takes priority over env var / config
let network = match cli.network {
    Some(ref name) => payg::network::resolve_network_config(name)?,
    None => consumer.resolve_network()?,
};
```

## Code locations to change (complete list)

| # | File | Line(s) | Change |
|---|------|---------|--------|
| 1 | `crates/payg/src/lib.rs` | 20-23 | Remove `BASE_CHAIN_ID`, `BASE_USDC_ADDRESS` constants |
| 2 | `crates/payg/src/lib.rs` | 38-69 | Add `network` param to `charge()` and `charge_with_config()` |
| 3 | `crates/payg/src/network.rs` | NEW | `NetworkConfig` struct + presets + resolver |
| 4 | `crates/payg/src/config.rs` | 24 | Add `network: Option<String>` to `ConsumerConfig` |
| 5 | `crates/payg/src/config.rs` | 96-104 | `rpc_url()` uses `network.default_rpc_url` |
| 6 | `crates/payg/src/config.rs` | NEW | `resolve_network()` method |
| 7 | `crates/payg/src/backend.rs` | 32-46 | Pass `NetworkConfig` to `from_config()` and backend constructors |
| 8 | `crates/payg/src/backend/x402.rs` | 16 | Change imports from constants to NetworkConfig |
| 9 | `crates/payg/src/backend/x402.rs` | 30-57 | Use network fields in `new()` and `charge()` |
| 10 | `crates/payg/src/backend/x402.rs` | 128-163 | Use `network.network_name` in `build_settle_request()` |
| 11 | `crates/payg-cli/src/cli.rs` | 12-19 | Add `--network` global flag |
| 12 | `crates/payg-cli/src/main.rs` | 16-20 | Thread `cli.network` to commands |
| 13 | `crates/payg-cli/src/commands/charge.rs` | 21-92 | Resolve network, pass to lib |
| 14 | `crates/payg-cli/src/commands/balance.rs` | 6-53 | Use `network.usdc_address`, display network name |
| 15 | `crates/payg-cli/src/commands/init.rs` | 90-130 | Write network to config, network-aware messaging |
| 16 | `crates/payg-cli/src/commands/address.rs` | 10-28 | Minor: resolve network for consistency |
| 17 | `README.md` | | Add network selection docs |

## References

- x402-rs network registry: `~/github-stars/x402-rs/x402-rs/crates/x402-types/src/networks.rs` (lines 148-170)
- x402-rs EIP-712 client signing: `~/github-stars/x402-rs/x402-rs/crates/chains/x402-chain-eip155/src/v1_eip155_exact/client.rs`
- Circle testnet USDC: `0x036CbD53842c5426634e7929541eC2318f3dCF7e` on Base Sepolia
- Circle USDC faucet: https://faucet.circle.com/
- Base Sepolia explorer: https://sepolia.basescan.org
- Existing solution doc: `docs/solutions/code-review/v1-multi-agent-review-resolution.md`
