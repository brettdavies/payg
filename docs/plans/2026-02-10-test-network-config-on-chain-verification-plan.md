---
title: "test: On-chain verification and integration tests for network config"
type: test
date: 2026-02-10
deepened: 2026-02-11
parent: docs/plans/2026-02-10-feat-configurable-network-for-testnet-support-plan.md
---

# On-Chain Verification and Integration Tests for Network Config

## Enhancement Summary

**Deepened on:** 2026-02-11
**Sections enhanced:** 8
**Research agents used:** 15 (security, performance, architecture, patterns, simplicity, agent-native, spec-flow, best-practices, framework-docs, repo-research, git-history, learnings, x402-testnet, erc3009-sepolia, tokio-testing)

### Key Improvements

1. **Corrected gas assumption**: x402 facilitator pays gas, not the signer. E2e test needs only USDC, not ETH.
2. **Facilitator is testnet-only**: `x402.org/facilitator` only supports Base Sepolia. Mainnet requires CDP or `facilitator.x402.rs`.
3. **Security hardening**: Added testnet assertion guard and test-specific charge ceiling.
4. **On-chain receipt verification**: Added `eth_getTransactionReceipt` step to close the "facilitator says ok" trust gap.
5. **Facilitator health check gate**: Prevents noisy CI failures on facilitator outage.
6. **Shared client via LazyLock**: Eliminates ~600ms-1.5s of redundant TLS handshakes.

### Critical Corrections to Original Plan

| Original Claim | Correction | Source |
|----------------|------------|--------|
| "Testnet ETH for gas (~$0.0001 per tx)" | x402 facilitator pays gas. E2e test needs only USDC. | x402 docs, spec-flow |
| "Uses `--test-threads=1` to prevent nonce conflicts" | ERC-3009 uses random 32-byte nonces, not sequential. `--test-threads=1` only needed for balance conflicts. | erc3009 research |
| "Reuse a single `reqwest::Client`" | Plan states this but implementation creates per-test clients. Use `LazyLock<reqwest::Client>`. | performance, tokio-testing |
| `charge_with_config("0.001 USDC", ...)` in AC | Implementation correctly uses `charge_raw()`. AC updated. | architecture |

---

## Overview

PR #3 (`feat/network-config`) adds configurable network selection with hardcoded `NetworkConfig` presets for Base mainnet and Base Sepolia. The 13 existing unit tests verify in-memory logic (parsing, resolution, URL validation) but none make real RPC calls. Before merging, we need to verify the hardcoded values match on-chain reality -- if the EIP-712 domain name is wrong, charges fail silently; if the USDC address is wrong, balances return 0.

This plan adds two categories of tests to the existing PR branch:

1. **On-chain verification tests** -- Read-only RPC calls verifying `NetworkConfig` values match deployed contracts (no wallet needed)
2. **End-to-end integration test** -- Actual testnet charge via x402 facilitator (requires funded wallet)

## Problem Statement

- `NetworkConfig` hardcodes 6 values per network that **have never been verified** against live chains
- The EIP-712 domain name differs between mainnet (`"USD Coin"`) and Sepolia (`"USDC"`) -- the most critical value to get right
- The USDC address, decimals, and chain ID must also be confirmed
- Default RPC URLs (`mainnet.base.org`, `sepolia.base.org`) must be reachable
- The v1 success metric -- "end-to-end testnet charge succeeds" -- has never been automated

### Research Insights: EIP-712 Domain Name Verification

**Confirmed via on-chain research:** The Base Sepolia USDC contract (`0x036CbD53842c5426634e7929541eC2318f3dCF7e`) is a `FiatTokenProxy` -> `FiatTokenV2_2` implementation. The EIP-712 domain name divergence (`"USDC"` on Sepolia vs `"USD Coin"` on mainnet) is a deployment-time configuration set during `initializeV2(newName)`, not a code difference. Using the wrong name produces `"FiatTokenV2: invalid signature"` reverts. Both PAYG's `network.rs` and x402-rs's `networks.rs` have this correctly configured.

**Confirmed via x402-rs:** Both V1 and V2 protocol versions use the same `sign_erc3009_authorization` function. The EIP-712 domain name is passed through `Eip3009SigningParams.extra.eip712_name`.

## Proposed Solution

### Test location

`crates/payg/tests/on_chain.rs` -- A single integration test file in the lib crate's `tests/` directory. Rust automatically treats files in `tests/` as integration tests that compile as separate crates.

**Research Insights: Architecture**

- **Lib crate's `tests/` is correct.** The e2e test calls `payg::charge_raw()` -- a lib API. Placing it in the CLI crate would be a layering violation.
- **One file is appropriate** for ~13 tests. The 200-line refactor trigger applies to production modules, not test files. When tests exceed ~15-20, migrate to `tests/on_chain/main.rs` sub-module pattern.
- **No `test-utils` module needed** for a 2-crate workspace with one consumer. Defer until 2+ consumers exist.

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

**Research Insights: `#[ignore]` vs Feature Flags**

| Approach | Verdict |
|----------|---------|
| `#[ignore]` | **Correct.** Test code always compiles (catches errors). `cargo test` is safe. Standard in alloy-rs, Foundry, ethers-rs. |
| `#[cfg(feature = "integration")]` | **Wrong.** Hides compile errors. Confuses contributors. Feature flag "leaks" into Cargo.toml. |
| Env-var guard with early return | **Supplementary.** Use for wallet-dependent tests (already done). Tests always "pass" so pollutes results. |

Best practice: `#[ignore]` as primary gate + env-var guard with skip messages for wallet-dependent tests. This is exactly what the e2e test does.

### Dependencies

Add to `crates/payg/Cargo.toml`:

```toml
[dev-dependencies]
tokio = { version = "1.35", features = ["macros", "rt"] }
reqwest = { version = "0.12", features = ["json"] }
```

- `tokio` with `macros` + `rt` enables `#[tokio::test]` (uses `current_thread` by default -- sufficient for sequential RPC calls)
- `reqwest` for raw JSON-RPC calls (consistent with existing `balance.rs` pattern, avoids pulling in alloy-provider for tests)

**Research Insights: Dependencies**

- **Pin `rand = "0.8"`** if tests import alloy-signer. alloy-signer-local v1.6 depends on rand 0.8. Mixing 0.8 and 0.9 causes trait incompatibility.
- **`serde_json`** is also needed as a dev-dep for JSON-RPC payloads (the plan omitted this but implementation correctly added it).
- **Raw reqwest is correct** over alloy Provider for read-only verification. Avoids adding alloy-provider/alloy-network to dev-deps. Consider alloy Provider only when tests need complex ABI decoding.

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
| `paused()` | `0x5c975abb` | ABI-encoded bool (NEW) |

ABI string decoding: offset (32 bytes) + length (32 bytes) + UTF-8 data (padded to 32 bytes).

## Technical Considerations

### Shared client via LazyLock

Use `std::sync::LazyLock` (stable since Rust 1.80, PAYG MSRV is 1.88) to share a single `reqwest::Client` across all tests. This eliminates ~600ms-1.5s of redundant TLS handshakes (12+ clients against 2 RPC hosts):

```rust
use std::sync::LazyLock;
use std::time::Duration;

static CLIENT: LazyLock<reqwest::Client> = LazyLock::new(|| {
    reqwest::Client::builder()
        .timeout(Duration::from_secs(15))
        .connect_timeout(Duration::from_secs(5))
        .build()
        .expect("failed to build HTTP client")
});
```

**Performance impact:** Reduces total suite runtime from 3.5-7.0s to 2.8-5.8s (parallel) by eliminating per-test TLS negotiation.

### Explicit timeouts

The 15s overall / 5s connect timeout values are appropriate for read-only tests. These are more aggressive than production timeouts (30s/10s) because test failures should surface quickly, not hang.

The e2e charge flows through `X402Backend`'s internal client (30s timeout), not the test's shared client. The test client only gates the balance preflight check.

### RPC URL override

Tests use `NetworkConfig.default_rpc_url` by default but respect a single env var override for CI flexibility:

```rust
fn rpc_url_for(network: &NetworkConfig) -> String {
    std::env::var("PAYG_TEST_RPC_URL")
        .unwrap_or_else(|_| network.default_rpc_url.to_string())
}
```

This lets CI use faster/higher-rate-limit RPCs (e.g., Alchemy, Infura) without modifying test code.

**Research Insights: Simplicity**

The per-network override pattern (`PAYG_TEST_RPC_URL_SEPOLIA`, `PAYG_TEST_RPC_URL_MAINNET`, `PAYG_TEST_RPC_URL`) added during code review is YAGNI. A single `PAYG_TEST_RPC_URL` env var is sufficient. Both networks use different default URLs already; the override is for CI rate-limit workarounds, not per-network routing.

### RPC rate limits

Public RPC endpoints (`mainnet.base.org`, `sepolia.base.org`) have rate limits (~10-30 req/s observed). Tests should:
- Reuse a single `reqwest::Client` via `LazyLock` (eliminates 12+ TCP connections)
- Not retry on failure (fail fast, not flaky)
- Run in parallel (12 requests is well under rate limits; `--test-threads=1` is unnecessary for read-only tests)

**Research Insights: Parallel Execution**

The original plan said "run sequentially." Research confirms parallel is safe for read-only tests: 6 requests per endpoint is under public rate limits. Only the e2e test needs `--test-threads=1` (or `serial_test` crate) if multiple wallet-dependent tests are added.

### Mainnet tests

Verifying Base mainnet values (`chain_id`, USDC contract, EIP-712 name) via read-only calls costs nothing and is safe. These tests should run alongside Sepolia tests since we ship both presets.

### Test wallet for end-to-end

The end-to-end charge test requires:
- `PAYG_PRIVATE_KEY` env var with a Base Sepolia funded key
- Testnet USDC from [Circle faucet](https://faucet.circle.com/) for x402 charges (20 USDC per request, rate limited to once per 2 hours)

~~Testnet ETH for gas~~ **Not needed.** The x402 facilitator pays gas on the signer's behalf. The signer only signs an ERC-3009 authorization locally; the facilitator submits the on-chain transaction and covers gas costs.

This test is gated on `PAYG_PRIVATE_KEY` being set -- it skips (not fails) when absent.

The e2e test charges to self (signer's own address) to avoid losing testnet USDC. ERC-3009 allows `from == to` transfers. Since the facilitator pays gas, USDC balance should be unchanged after a self-charge.

### Facilitator health check gate (NEW)

Before attempting the charge, check facilitator reachability with a lightweight HTTP request. If unreachable, skip cleanly instead of hard-failing with a 30-second timeout:

```rust
// Quick facilitator health check (5s timeout)
let health_client = reqwest::Client::builder()
    .timeout(Duration::from_secs(5))
    .build()
    .expect("failed to build health check client");
let facilitator_url = config.facilitator_url
    .as_deref()
    .unwrap_or(payg::DEFAULT_FACILITATOR_URL);
if health_client.get(facilitator_url).send().await.is_err() {
    eprintln!("Skipping: x402 facilitator at {facilitator_url} is unreachable");
    return;
}
```

**Rationale:** The e2e test already has skip gates for missing wallet and insufficient balance. A facilitator gate prevents noisy CI failures on facilitator outage.

### Research Insights: x402 Facilitator

| Facilitator | URL | Networks | Auth |
|-------------|-----|----------|------|
| x402.org (Coinbase) | `https://x402.org/facilitator` | Base Sepolia, Solana Devnet **only** | None |
| CDP (Coinbase) | `https://api.cdp.coinbase.com/platform/v2/x402` | Testnet + Mainnet | CDP API keys |
| facilitator.x402.rs (Rust community) | `https://facilitator.x402.rs` | Multiple chains | None |

**Critical discovery:** `x402.org/facilitator` is **testnet-only**. It does not support Base mainnet. For mainnet charges, PAYG must use CDP or `facilitator.x402.rs`. This is documented in a Firecrawl issue (#2212) and confirmed by x402.org ecosystem docs.

### On-chain receipt verification (NEW)

After `charge_raw()` returns a receipt, verify the transaction actually landed on-chain:

```rust
// Verify tx landed on-chain (read-only, costs nothing)
let receipt = rpc_call(
    &CLIENT, &rpc_url,
    "eth_getTransactionReceipt",
    serde_json::json!([&charge_receipt.tx_hash]),
).await;
let status = receipt["status"].as_str().expect("receipt missing status");
assert_eq!(status, "0x1", "on-chain tx did not succeed");
```

**Rationale:** Without this, the test proves "the facilitator said it worked" but not "the chain confirms it worked." This is ~10 lines and uses the existing `rpc_call` helper.

### Security guards (NEW)

Two defense-in-depth measures for the e2e test:

```rust
// 1. Testnet-only assertion (prevents accidental mainnet charges)
assert!(network.is_testnet, "e2e charge tests MUST run on testnet only");

// 2. Test-specific charge ceiling (independent of library safety ceiling)
const E2E_MAX_CHARGE_USDC: u64 = 10_000; // 0.01 USDC absolute ceiling
assert!(
    charge_amount <= E2E_MAX_CHARGE_USDC,
    "test charge {charge_amount} exceeds test ceiling {E2E_MAX_CHARGE_USDC}"
);
```

**Rationale:** `charge_raw()` bypasses the library's safety ceiling. The testnet assertion prevents refactoring accidents. Both are single-line, zero-cost guards.

### Balance preflight for e2e

Before attempting a charge, the e2e test checks the wallet's USDC balance. If insufficient, it prints a message pointing to the Circle faucet and skips (doesn't fail):

```rust
if usdc_balance < min_charge_amount {
    eprintln!(
        "Skipping: insufficient USDC on base-sepolia. \
         Get testnet USDC from https://faucet.circle.com/ \
         (no ETH needed — x402 facilitator pays gas)"
    );
    return;
}
```

**Research Insights:** The skip message now clarifies that ETH for gas is NOT needed for x402 charges. This prevents user confusion about needing both testnet USDC and testnet ETH.

### Nonce handling

ERC-3009 `transferWithAuthorization` uses **random 32-byte nonces** (generated via `rand` crate in x402-chain-eip155), not sequential Ethereum transaction nonces. Collision probability is astronomically low (~2^-256). This means:

- No nonce conflicts between parallel test runs
- No stale nonce from previous failed tests
- `--test-threads=1` is only needed for balance conflicts (same wallet spending simultaneously), not nonce conflicts

### Error handling in tests

Tests that make RPC calls can fail due to network issues. They should:
- Use `.expect("descriptive message")` not `.unwrap()` for clear failure diagnostics
- NOT retry -- transient network failures should surface as test failures
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

### Research Insights: `tokio::test` Runtime

- **No built-in timeout attribute.** `#[tokio::test]` does not support `#[timeout]`. Use client-level timeouts (already done) and `cargo-nextest` for CI process-level timeouts.
- **`current_thread` is correct.** Matches PAYG's production runtime. Each `#[tokio::test]` creates its own runtime.
- **`LazyLock` persists across tests** since they share a process. `tokio::sync::OnceCell` is available for async init if needed.

## Acceptance Criteria

### On-chain verification (both networks)

- [x] Verify `eth_chainId` matches `NetworkConfig.chain_id` for both networks
- [x] Verify USDC contract has deployed code at `NetworkConfig.usdc_address` for both networks
- [x] Verify `decimals()` returns 6 for both USDC contracts
- [x] Verify `name()` returns `"USD Coin"` on mainnet and `"USDC"` on Sepolia
- [x] Verify `version()` returns `"2"` on both networks
- [x] Verify default RPC URLs are reachable (`eth_blockNumber` succeeds)

### End-to-end charge (Sepolia only, gated on funded wallet)

- [x] `payg::charge_raw(amount, recipient, ...)` succeeds on Base Sepolia
- [x] Receipt contains valid tx_hash (0x-prefixed, 66 chars)
- [x] Test skips cleanly when `PAYG_PRIVATE_KEY` is not set
- [x] On-chain receipt confirms tx succeeded (`status == "0x1"`) (NEW)
- [x] Test skips cleanly when facilitator is unreachable (NEW)
- [x] Testnet-only assertion guard present (NEW)
- [x] Test-specific charge ceiling constant present (NEW)

### Infrastructure

- [x] `cargo test` still runs only fast unit tests (13 tests, <1s)
- [x] `cargo test -- --ignored` runs on-chain verification tests
- [x] `cargo clippy` clean with dev-dependencies
- [x] No new production dependencies added
- [x] Shared `reqwest::Client` via `LazyLock` (NEW)

## Implementation Plan

### Phase 1: Test infrastructure

**File:** `crates/payg/Cargo.toml`

Add `[dev-dependencies]` section with tokio, reqwest, and serde_json.

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
//!
//! For e2e tests (requires funded wallet):
//!
//!   PAYG_PRIVATE_KEY=0x... cargo test e2e -- --ignored

use std::sync::LazyLock;
use std::time::Duration;
use payg::network::{BASE_MAINNET, BASE_SEPOLIA, NetworkConfig};

static CLIENT: LazyLock<reqwest::Client> = LazyLock::new(|| {
    reqwest::Client::builder()
        .timeout(Duration::from_secs(15))
        .connect_timeout(Duration::from_secs(5))
        .build()
        .expect("failed to build HTTP client")
});

// Shared helper: make a JSON-RPC call
async fn rpc_call(client: &reqwest::Client, rpc_url: &str, method: &str, params: serde_json::Value) -> serde_json::Value { ... }

// Shared helper: call a view function on a contract
async fn eth_call_hex(client: &reqwest::Client, rpc_url: &str, to: &str, selector: &str) -> String { ... }

// Shared helper: decode ABI-encoded string
fn decode_abi_string(hex: &str) -> String { ... }

// Shared helper: RPC URL with env var override
fn rpc_url_for(network: &NetworkConfig) -> String {
    std::env::var("PAYG_TEST_RPC_URL")
        .unwrap_or_else(|_| network.default_rpc_url.to_string())
}

// Parameterized verify helpers (one per assertion, called by thin wrappers)
async fn verify_chain_id(network: &NetworkConfig) { ... }
async fn verify_usdc_contract_exists(network: &NetworkConfig) { ... }
async fn verify_usdc_decimals(network: &NetworkConfig) { ... }
async fn verify_usdc_name(network: &NetworkConfig) { ... }
async fn verify_usdc_version(network: &NetworkConfig) { ... }
async fn verify_rpc_reachable(network: &NetworkConfig) { ... }

// === Base Sepolia Tests ===

#[tokio::test] #[ignore] async fn sepolia_chain_id_matches() { verify_chain_id(&BASE_SEPOLIA).await; }
#[tokio::test] #[ignore] async fn sepolia_usdc_contract_exists() { verify_usdc_contract_exists(&BASE_SEPOLIA).await; }
#[tokio::test] #[ignore] async fn sepolia_usdc_decimals_is_6() { verify_usdc_decimals(&BASE_SEPOLIA).await; }
#[tokio::test] #[ignore] async fn sepolia_usdc_name_matches_eip712() { verify_usdc_name(&BASE_SEPOLIA).await; }
#[tokio::test] #[ignore] async fn sepolia_usdc_version_is_2() { verify_usdc_version(&BASE_SEPOLIA).await; }
#[tokio::test] #[ignore] async fn sepolia_rpc_is_reachable() { verify_rpc_reachable(&BASE_SEPOLIA).await; }

// === Base Mainnet Tests ===

#[tokio::test] #[ignore] async fn mainnet_chain_id_matches() { verify_chain_id(&BASE_MAINNET).await; }
#[tokio::test] #[ignore] async fn mainnet_usdc_contract_exists() { verify_usdc_contract_exists(&BASE_MAINNET).await; }
#[tokio::test] #[ignore] async fn mainnet_usdc_decimals_is_6() { verify_usdc_decimals(&BASE_MAINNET).await; }
#[tokio::test] #[ignore] async fn mainnet_usdc_name_matches_eip712() { verify_usdc_name(&BASE_MAINNET).await; }
#[tokio::test] #[ignore] async fn mainnet_usdc_version_is_2() { verify_usdc_version(&BASE_MAINNET).await; }
#[tokio::test] #[ignore] async fn mainnet_rpc_is_reachable() { verify_rpc_reachable(&BASE_MAINNET).await; }

// === End-to-end charge (gated on PAYG_PRIVATE_KEY) ===

/// Maximum charge amount for e2e tests. Intentionally very small to prevent
/// accidental wallet drain. DO NOT increase without security review.
const E2E_MAX_CHARGE_USDC: u64 = 10_000; // 0.01 USDC absolute ceiling

#[tokio::test]
#[ignore]
async fn e2e_charge_sepolia() {
    // Gate 1: Wallet
    let Ok(key) = std::env::var("PAYG_PRIVATE_KEY") else {
        eprintln!("Skipping e2e_charge_sepolia: PAYG_PRIVATE_KEY not set");
        return;
    };

    let network = &BASE_SEPOLIA;
    assert!(network.is_testnet, "e2e charge tests MUST run on testnet only");

    // Gate 2: Facilitator health check
    let facilitator_url = payg::DEFAULT_FACILITATOR_URL;
    let health_client = reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .expect("failed to build health check client");
    if health_client.get(facilitator_url).send().await.is_err() {
        eprintln!("Skipping: x402 facilitator at {facilitator_url} is unreachable");
        return;
    }

    // Gate 3: USDC balance
    let charge_amount = 1000u64; // 0.001 USDC
    assert!(charge_amount <= E2E_MAX_CHARGE_USDC, "test charge exceeds ceiling");
    // ... check USDC balance, skip if insufficient with faucet message ...
    // ... include: "(no ETH needed — x402 facilitator pays gas)"

    // Charge to self
    let config = payg::ConsumerConfig::default();
    let signer = payg::wallet::load_wallet(&config, None).expect("wallet load failed");
    let recipient = signer.address();
    let amount = alloy_primitives::U256::from(charge_amount);
    let receipt = payg::charge_raw(amount, recipient, &config, network, signer)
        .await
        .expect("charge_raw failed on base-sepolia");

    // Verify receipt format
    assert!(receipt.tx_hash.starts_with("0x"), "tx_hash missing 0x prefix");
    assert_eq!(receipt.tx_hash.len(), 66, "tx_hash wrong length");

    // Verify on-chain confirmation (NEW)
    let rpc_url = rpc_url_for(network);
    let tx_receipt = rpc_call(
        &CLIENT, &rpc_url,
        "eth_getTransactionReceipt",
        serde_json::json!([&receipt.tx_hash]),
    ).await;
    let status = tx_receipt["status"].as_str()
        .expect("transaction receipt missing status field");
    assert_eq!(status, "0x1", "on-chain tx did not succeed");
}
```

### Phase 3: Run and verify

```bash
# Verify unit tests still pass
cargo test

# Run on-chain verification (read-only, no wallet)
cargo test -- --ignored

# Run e2e with funded wallet
PAYG_PRIVATE_KEY=0x... cargo test e2e -- --ignored

# Both together
cargo test -- --include-ignored
```

## E2E Charge Flow (Verified by Research)

```
PAYG_PRIVATE_KEY env var
  -> payg::wallet::load_wallet() -> PrivateKeySigner
     -> payg::charge_raw(amount, recipient, config, network, signer)
        -> Backend::from_config() -> X402Backend::new(facilitator_url, signer, network)
           -> X402Backend::charge(&request)
              1. Build Eip3009SigningParams:
                 - chain_id: 84532 (Base Sepolia)
                 - usdc_address: 0x036CbD53842c5426634e7929541eC2318f3dCF7e
                 - eip712_name: "USDC" (NOT "USD Coin" -- Sepolia-specific)
                 - eip712_version: "2"
                 - nonce: random 32 bytes (no collision risk)
                 - validBefore: now + 120s
              2. sign_erc3009_authorization() via x402-chain-eip155
              3. build_settle_request() with ExactEvmPayload + PaymentRequirements
              4. POST /settle to https://x402.org/facilitator
                 (facilitator pays gas, submits on-chain tx)
              5. Parse response: { success: true, transaction: "0x..." }
              6. Return ChargeReceipt { tx_hash }
```

**Timing estimate:** ~2.2-2.7s typical (local signing <5ms, facilitator round-trip 200-500ms, block inclusion ~2s). Worst case: 5-10s.

## Security Analysis

| Risk | Severity | Mitigation |
|------|----------|------------|
| Accidental mainnet charge | HIGH | `assert!(network.is_testnet)` guard |
| `charge_raw()` bypasses safety ceiling | HIGH | `E2E_MAX_CHARGE_USDC` test ceiling constant |
| Malicious RPC URL steals private key | NONE | Architectural guarantee: x402 never sends key over wire, only signatures |
| Test private key committed to git | LOW | `.gitignore` covers `.env*`, `*.key`, `keyfile.json`. No keys in code. |
| Wallet drain via repeated runs | LOW | Charge-to-self returns USDC. Gas paid by facilitator. |
| Facilitator observes authorization | LOW | Authorization is single-use (random nonce), time-limited (120s), and charge-to-self. |

## Performance Estimates

| Scenario | Estimated Runtime |
|----------|------------------|
| Read-only tests, parallel, shared client | 2.8-5.8s |
| Read-only tests, parallel, per-test client | 3.5-7.0s |
| Read-only tests, sequential | 5.5-11.0s |
| E2e charge (single test) | 2.2-2.7s typical, 5-10s worst |
| Full suite (parallel read + e2e) | 5.0-15.8s |

## What else should be in PR #3?

After reviewing the acceptance criteria from the network config plan and the current PR state:

1. **Tests (this plan)** -- Required. The PR introduces hardcoded values that need verification.
2. **README** -- Done. Completed on `feat/readme` branch, merged.
3. **CI configuration** -- Deferred. The project has no `.github/workflows/` yet. CI is a separate concern.

The PR is feature-complete for its scope. Tests are the only missing piece.

## Files to create/modify

| # | File | Action | Description |
|---|------|--------|-------------|
| 1 | `crates/payg/Cargo.toml` | Modify | Add `[dev-dependencies]` with tokio + reqwest + serde_json |
| 2 | `crates/payg/tests/on_chain.rs` | Modify | Add security guards, facilitator health check, on-chain receipt verification, LazyLock client |

## Deferred (from SpecFlow analysis, updated with research)

The SpecFlow analysis identified additional testing opportunities. These are deferred to keep this PR focused:

- **Cross-network signature rejection test** -- Verify that signing with mainnet EIP-712 params fails on Sepolia. Valuable but requires facilitator integration and funded wallet.
- **ETH backend integration test** -- Direct ETH transfer on testnet. Different gas model (sender pays gas), separate wallet funding. Better as a separate PR when ETH backend is the focus.
- **CI workflow** -- `.github/workflows/ci.yml` with read-only tests on PR, wallet tests nightly. Separate PR.
- **USDC `paused()` check** -- `0x5c975abb` selector returns bool. Would provide early warning before e2e hits paused contract. Low priority (Sepolia pauses are rare).
- **Post-charge balance delta assertion** -- Verify USDC balance unchanged after self-charge. Low priority (tx receipt verification is more reliable).
- **Error path coverage** -- Facilitator rejection, timeout, rate limit, non-JSON response. Requires mock facilitator or deliberate error injection. Deferred to mock facilitator work.
- **`charge_with_config()` e2e test** -- Exercises full public API including string parsing and safety ceiling. Lower priority than `charge_raw()` e2e.

**Removed from deferred (per simplicity review):**
- ~~Mock facilitator~~ -- `#[ignore]` + facilitator health check is sufficient for a 2-crate project.
- ~~Contract upgrade detection~~ -- Monitor manually. USDC proxy upgrades are rare and would break the read-only tests.
- ~~Config precedence integration tests~~ -- Already covered by unit tests in `network.rs` and `config.rs`.

## References

- Existing RPC call pattern: `crates/payg-cli/src/commands/balance.rs:64-136`
- NetworkConfig presets: `crates/payg/src/network.rs:20-38`
- x402 backend: `crates/payg/src/backend/x402.rs`
- Solution doc (alloy gotchas): `docs/solutions/build-errors/rust-workspace-dependency-conflicts-and-feature-flags.md`
- Solution doc (review batch 1): `docs/solutions/code-review/network-config-review-and-systematic-fix-batching.md`
- Solution doc (review batch 2): `docs/solutions/code-review/second-review-pass-agent-parity-fixes.md`
- x402-rs networks registry: `~/github-stars/x402-rs/x402-rs/crates/chains/x402-chain-eip155/src/networks.rs`
- Circle USDC faucet: https://faucet.circle.com/
- Base Sepolia explorer: https://sepolia.basescan.org
- Base Mainnet explorer: https://basescan.org
- x402 facilitator: https://x402.org/facilitator (testnet-only)
- x402 ecosystem: https://x402.org/ecosystem
- CDP facilitator (mainnet): https://docs.cdp.coinbase.com/x402/network-support

## Research Output Files

Detailed findings from each research agent are archived at:

| Agent | Output |
|-------|--------|
| Security Sentinel | `/tmp/payg-deepen-security.md` |
| Performance Oracle | `/tmp/payg-deepen-performance.md` |
| Architecture Strategist | `/tmp/payg-deepen-architecture.md` |
| Pattern Recognition | `/tmp/payg-deepen-patterns.md` |
| Code Simplicity | `/tmp/payg-deepen-simplicity.md` |
| Agent-Native | `/tmp/payg-deepen-agent-native.md` |
| Spec Flow Analyzer | `/tmp/payg-deepen-specflow.md` |
| Best Practices | `/tmp/payg-deepen-best-practices.md` |
| Framework Docs | `/tmp/payg-deepen-framework-docs.md` |
| Repo Research | `/tmp/payg-deepen-repo-patterns.md` |
| Git History | `/tmp/payg-deepen-git-history.md` |
| Learnings | `/tmp/payg-deepen-learnings.md` |
| x402 Testnet | `/tmp/payg-deepen-x402-testnet.md` |
| ERC-3009 Sepolia | `/tmp/payg-deepen-erc3009-sepolia.md` |
| Tokio Testing | `/tmp/payg-deepen-tokio-testing.md` |
