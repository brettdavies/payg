---
title: "test: On-chain verification and integration tests for network config"
type: test
date: 2026-02-10
parent: docs/plans/2026-02-10-feat-configurable-network-for-testnet-support-plan.md
---

# On-Chain Verification and Integration Tests for Network Config

## Overview

PR #3 (`feat/network-config`) adds configurable network selection with hardcoded `NetworkConfig` presets for Base mainnet and Base Sepolia. The 13 existing unit tests verify in-memory logic (parsing, resolution, URL validation) but none make real RPC calls. Before merging, we need to verify the hardcoded values match on-chain reality — if the EIP-712 domain name is wrong, charges fail silently; if the USDC address is wrong, balances return 0.

This plan adds two categories of tests to the existing PR branch:

1. **On-chain verification tests** — Read-only RPC calls verifying `NetworkConfig` values match deployed contracts (no wallet needed)
2. **End-to-end integration test** — Actual testnet charge via x402 facilitator (requires funded wallet)

## Problem Statement

- `NetworkConfig` hardcodes 6 values per network that **have never been verified** against live chains
- The EIP-712 domain name differs between mainnet (`"USD Coin"`) and Sepolia (`"USDC"`) — the most critical value to get right
- The USDC address, decimals, and chain ID must also be confirmed
- Default RPC URLs (`mainnet.base.org`, `sepolia.base.org`) must be reachable
- The v1 success metric — "end-to-end testnet charge succeeds" — has never been automated

## Proposed Solution

### Test location

`crates/payg/tests/on_chain.rs` — A single integration test file in the lib crate's `tests/` directory. Rust automatically treats files in `tests/` as integration tests that compile as separate crates.

### Gating strategy: `#[ignore]`

All network-dependent tests use `#[ignore]` so that `cargo test` remains fast and offline:

```bash
# Fast unit tests (always, no network)
cargo test

# On-chain verification (network required, no wallet)
cargo test -- --ignored

# Both together
cargo test -- --include-ignored
```

This is simpler than a feature flag and is the standard Rust pattern for slow/network tests.

### Dependencies

Add to `crates/payg/Cargo.toml`:

```toml
[dev-dependencies]
tokio = { version = "1.35", features = ["macros", "rt"] }
reqwest = { version = "0.12", features = ["json"] }
```

- `tokio` with `macros` + `rt` enables `#[tokio::test]` (uses `current_thread` by default — sufficient for sequential RPC calls)
- `reqwest` for raw JSON-RPC calls (consistent with existing `balance.rs` pattern, avoids pulling in alloy-provider for tests)

### Test approach: Raw JSON-RPC

Use the same raw `eth_call` pattern already established in `balance.rs` rather than alloy-provider. This:
- Adds zero new transitive dependencies (reqwest is already a dev-dep transitively)
- Is consistent with the existing codebase
- Avoids the alloy Provider trait import complexity documented in our solution docs
- Works for read-only queries without a wallet

### Function selectors for ERC-20 verification

| Function | Selector | Returns |
|----------|----------|---------|
| `name()` | `0x06fdde03` | ABI-encoded string |
| `version()` | `0x54fd4d50` | ABI-encoded string |
| `decimals()` | `0x313ce567` | ABI-encoded uint8 |

ABI string decoding: offset (32 bytes) + length (32 bytes) + UTF-8 data (padded to 32 bytes).

## Technical Considerations

### Explicit timeouts

The shared `reqwest::Client` must have explicit timeouts to prevent tests from hanging when RPCs are unresponsive:

```rust
let client = reqwest::Client::builder()
    .timeout(Duration::from_secs(15))
    .connect_timeout(Duration::from_secs(5))
    .build()
    .expect("failed to build HTTP client");
```

This matches the timeout pattern already established in `x402.rs` and `balance.rs`.

### RPC URL override

Tests use `NetworkConfig.default_rpc_url` by default but respect env var overrides for CI flexibility:

```rust
fn rpc_url_for(network: &NetworkConfig) -> String {
    std::env::var("PAYG_TEST_RPC_URL")
        .unwrap_or_else(|_| network.default_rpc_url.to_string())
}
```

This lets CI use faster/higher-rate-limit RPCs (e.g., Alchemy, Infura) without modifying test code.

### RPC rate limits

Public RPC endpoints (`mainnet.base.org`, `sepolia.base.org`) have rate limits. Tests should:
- Reuse a single `reqwest::Client` across all assertions
- Not retry on failure (fail fast, not flaky)
- Run sequentially (no parallel RPC calls in tests)

### Mainnet tests

Verifying Base mainnet values (`chain_id`, USDC contract, EIP-712 name) via read-only calls costs nothing and is safe. These tests should run alongside Sepolia tests since we ship both presets.

### Test wallet for end-to-end

The end-to-end charge test requires:
- `PAYG_PRIVATE_KEY` env var with a Base Sepolia funded key
- Testnet ETH for gas (~$0.0001 per tx)
- Testnet USDC from [Circle faucet](https://faucet.circle.com/) for x402 charges

This test is gated on `PAYG_PRIVATE_KEY` being set — it skips (not fails) when absent.

The e2e test charges to self (signer's own address) to avoid losing testnet USDC and to enable balance pre/post assertions. Uses `--test-threads=1` when running wallet tests to prevent nonce conflicts.

### Balance preflight for e2e

Before attempting a charge, the e2e test checks the wallet's USDC balance. If insufficient, it prints a message pointing to the Circle faucet and skips (doesn't fail):

```rust
if usdc_balance < min_charge_amount {
    eprintln!("Skipping: insufficient USDC. Get testnet USDC from https://faucet.circle.com/");
    return;
}
```

### Error handling in tests

Tests that make RPC calls can fail due to network issues. They should:
- Use `.expect("descriptive message")` not `.unwrap()` for clear failure diagnostics
- NOT retry — transient network failures should surface as test failures
- Print the RPC URL on failure for debugging

### Test naming convention

Tests are prefixed with the network name for filterability:

```bash
# Run only Sepolia read-only tests
cargo test sepolia -- --ignored

# Run only mainnet read-only tests
cargo test mainnet -- --ignored

# Run only e2e tests (requires wallet)
cargo test e2e -- --ignored
```

## Acceptance Criteria

### On-chain verification (both networks)

- [x] Verify `eth_chainId` matches `NetworkConfig.chain_id` for both networks
- [x] Verify USDC contract has deployed code at `NetworkConfig.usdc_address` for both networks
- [x] Verify `decimals()` returns 6 for both USDC contracts
- [x] Verify `name()` returns `"USD Coin"` on mainnet and `"USDC"` on Sepolia
- [x] Verify `version()` returns `"2"` on both networks
- [x] Verify default RPC URLs are reachable (`eth_blockNumber` succeeds)

### End-to-end charge (Sepolia only, gated on funded wallet)

- [x] `payg::charge_with_config("0.001 USDC", recipient, ...)` succeeds on Base Sepolia
- [x] Receipt contains valid tx_hash (0x-prefixed, 66 chars)
- [x] Test skips cleanly when `PAYG_PRIVATE_KEY` is not set

### Infrastructure

- [x] `cargo test` still runs only fast unit tests (13 tests, <1s)
- [x] `cargo test -- --ignored` runs on-chain verification tests
- [x] `cargo clippy` clean with dev-dependencies
- [x] No new production dependencies added

## Implementation Plan

### Phase 1: Test infrastructure

**File:** `crates/payg/Cargo.toml`

Add `[dev-dependencies]` section with tokio and reqwest.

### Phase 2: On-chain verification tests

**File:** `crates/payg/tests/on_chain.rs`

Structure:

```rust
//! On-chain verification tests for NetworkConfig presets.
//!
//! These tests make read-only RPC calls to verify that hardcoded values
//! in NetworkConfig match deployed contract reality. Run with:
//!
//!   cargo test -- --ignored

use std::time::Duration;
use payg::network::{BASE_MAINNET, BASE_SEPOLIA, NetworkConfig};

// Shared helper: make a JSON-RPC call
async fn rpc_call(client: &reqwest::Client, rpc_url: &str, method: &str, params: serde_json::Value) -> serde_json::Value { ... }

// Shared helper: call a view function on a contract
async fn eth_call_hex(client: &reqwest::Client, rpc_url: &str, to: &str, selector: &str) -> String { ... }

// Shared helper: decode ABI-encoded string
fn decode_abi_string(hex: &str) -> String { ... }

// === Base Sepolia Tests ===

#[tokio::test]
#[ignore]
async fn sepolia_chain_id_matches() { ... }

#[tokio::test]
#[ignore]
async fn sepolia_usdc_contract_exists() { ... }

#[tokio::test]
#[ignore]
async fn sepolia_usdc_decimals_is_6() { ... }

#[tokio::test]
#[ignore]
async fn sepolia_usdc_name_matches_eip712() { ... }

#[tokio::test]
#[ignore]
async fn sepolia_usdc_version_is_2() { ... }

#[tokio::test]
#[ignore]
async fn sepolia_rpc_is_reachable() { ... }

// === Base Mainnet Tests ===

#[tokio::test]
#[ignore]
async fn mainnet_chain_id_matches() { ... }

#[tokio::test]
#[ignore]
async fn mainnet_usdc_contract_exists() { ... }

#[tokio::test]
#[ignore]
async fn mainnet_usdc_decimals_is_6() { ... }

#[tokio::test]
#[ignore]
async fn mainnet_usdc_name_matches_eip712() { ... }

#[tokio::test]
#[ignore]
async fn mainnet_usdc_version_is_2() { ... }

#[tokio::test]
#[ignore]
async fn mainnet_rpc_is_reachable() { ... }

// === End-to-end charge (gated on PAYG_PRIVATE_KEY) ===

#[tokio::test]
#[ignore]
async fn e2e_charge_sepolia() {
    let Ok(key) = std::env::var("PAYG_PRIVATE_KEY") else {
        eprintln!("Skipping e2e_charge_sepolia: PAYG_PRIVATE_KEY not set");
        return;
    };
    // ... charge 0.001 USDC to self, verify receipt
}
```

### Phase 3: Run and verify

```bash
# Verify unit tests still pass
cargo test

# Run on-chain verification
cargo test -- --ignored

# Both together
cargo test -- --include-ignored
```

## What else should be in PR #3?

After reviewing the acceptance criteria from the network config plan and the current PR state:

1. **Tests (this plan)** — Required. The PR introduces hardcoded values that need verification.
2. **README** — Deferred. No README exists yet. Creating one is a separate task that covers the entire project, not just network config.
3. **CI configuration** — Deferred. The project has no `.github/workflows/` yet. CI is a separate concern that should cover linting, testing, and both feature flag combinations.

The PR is feature-complete for its scope. Tests are the only missing piece.

## Files to create/modify

| # | File | Action | Description |
|---|------|--------|-------------|
| 1 | `crates/payg/Cargo.toml` | Modify | Add `[dev-dependencies]` with tokio + reqwest |
| 2 | `crates/payg/tests/on_chain.rs` | Create | Integration tests with `#[ignore]` |

## Deferred (from SpecFlow analysis)

The SpecFlow analysis (`/tmp/payg-specflow-testing.md`) identified additional testing opportunities. These are deferred to keep this PR focused:

- **Cross-network signature rejection test** — Verify that signing with mainnet EIP-712 params fails on Sepolia. Valuable but requires facilitator integration and funded wallet.
- **ETH backend integration test** — Direct ETH transfer on testnet. Different gas model, separate wallet funding. Better as a separate PR when ETH backend is the focus.
- **Mock facilitator** — Offline testing without network calls. Separate infrastructure concern.
- **CI workflow** — `.github/workflows/ci.yml` with read-only tests on PR, wallet tests nightly. Separate PR.
- **Config precedence integration tests** — CLI flag > env var > config file > default. These are unit-testable and some already exist in `network.rs` and `config.rs`.
- **Contract upgrade detection** — USDC proxy upgrades could change ABI. Low probability, monitor manually.

## References

- Existing RPC call pattern: `crates/payg-cli/src/commands/balance.rs:64-136`
- NetworkConfig presets: `crates/payg/src/network.rs:20-38`
- Solution doc (alloy gotchas): `docs/solutions/build-errors/rust-workspace-dependency-conflicts-and-feature-flags.md`
- Circle USDC faucet: https://faucet.circle.com/
- Base Sepolia explorer: https://sepolia.basescan.org
- Base Mainnet explorer: https://basescan.org
