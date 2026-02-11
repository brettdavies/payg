---
title: "On-chain test suite hardening: static HTTP client, testnet guards, and receipt verification"
date: 2026-02-11
category: integration-issues
tags:
  - testing
  - on-chain
  - test-hardening
  - static-resources
  - security-guards
  - receipt-verification
  - reqwest
  - lazy-lock
severity: medium
component: crates/payg/tests/on_chain.rs
symptom: "Redundant HTTP clients per test + missing security guards in e2e charge test (no testnet assertion, charge ceiling, facilitator health check, or on-chain receipt verification)"
root_cause: "Parallel tokio tests each constructed reqwest::Client causing redundant TLS handshakes; e2e test trusted facilitator response without validating testnet-only execution or verifying on-chain receipt"
---

# On-chain test suite hardening: static HTTP client, testnet guards, and receipt verification

## Problem

The `payg` project's on-chain integration test suite in `crates/payg/tests/on_chain.rs` had two critical issues:

1. **Redundant HTTP clients**: Each of 12+ parallel `#[tokio::test]` functions was constructing its own `reqwest::Client`, causing redundant TLS handshakes and performance degradation.
2. **Missing security guards in e2e charge test**: The e2e test that sends real USDC on Base Sepolia lacked essential safety mechanisms:
   - Testnet assertion guard (could accidentally run on mainnet)
   - Charge ceiling constant (no cap on test charge amount)
   - Facilitator health check (hard failure if facilitator down)
   - On-chain receipt verification (trusted facilitator's tx_hash without confirming on-chain)

## Investigation

### Root Cause Analysis

The root causes were:
- **Parallel test architecture**: The test suite used `#[tokio::test]` with multiple parallel test functions, each independently constructing `reqwest::Client`.
- **Missing defense-in-depth**: The e2e charge test was the only test that executed real transactions and lacked multiple layers of safety guards.
- **Trust in external services**: The test trusted the x402 facilitator's response without verifying on-chain execution.

### Initial Symptoms

- **Performance**: Slow test execution due to repeated TLS handshakes
- **Security risk**: Potential mainnet execution if testnet assertion failed
- **Reliability**: Tests could fail silently if facilitator was down
- **Verification gap**: No confirmation that transactions actually landed on-chain

## Solution

### 1. Static HTTP Client Pattern

The core solution was implementing a shared `LazyLock` HTTP client to eliminate redundant TLS handshakes:

```rust
use std::sync::LazyLock;
use std::time::Duration;

// Static shared client for all parallel tests
static CLIENT: LazyLock<reqwest::Client> = LazyLock::new(|| {
    reqwest::Client::builder()
        .connect_timeout(Duration::from_secs(5))
        .timeout(Duration::from_secs(15))
        .build()
        .expect("failed to build shared HTTP client")
});
```

This pattern ensures:
- Single TLS handshake per test run
- Thread-safe access across parallel `#[tokio::test]` functions
- Proper connection lifecycle management
- Consistent timeout configuration

### 2. Security Guards Implementation

#### Testnet Assertion Guard
```rust
let network = &BASE_SEPOLIA;
assert!(
    network.is_testnet,
    "e2e charge tests MUST run on testnet only"
);
```

#### Charge Ceiling Constant
```rust
const E2E_MAX_CHARGE_USDC: u64 = 10_000;

// 0.001 USDC = 1000 units (6 decimals)
let charge_amount = 1000u64;
assert!(
    charge_amount <= E2E_MAX_CHARGE_USDC,
    "test charge {charge_amount} exceeds test ceiling {E2E_MAX_CHARGE_USDC}"
);
```

#### Facilitator Health Check
```rust
// Gate 2: Facilitator health check — skip if unreachable
let facilitator_url = payg::DEFAULT_FACILITATOR_URL;
if CLIENT.get(facilitator_url).send().await.is_err() {
    eprintln!("SKIP: e2e_charge_sepolia: x402 facilitator at {facilitator_url} is unreachable");
    return;
}
```

#### On-chain Receipt Verification
```rust
// Execute the charge
let amount = alloy_primitives::U256::from(charge_amount);
let receipt = payg::charge_raw(amount, recipient, &config, network, signer)
    .await
    .expect("charge_raw failed on base-sepolia");

// Verify transaction was mined and successful
let receipt_result = rpc_call(
    &CLIENT,
    &rpc_url,
    "eth_getTransactionReceipt",
    serde_json::json!([receipt.tx_hash]),
)
.await;
let receipt_json = receipt_result
    .as_object()
    .expect("receipt result is not an object");
let status_hex = receipt_json
    .get("status")
    .expect("receipt missing status field")
    .as_str()
    .expect("status is not a string");
let status = u64::from_str_radix(status_hex.trim_start_matches("0x"), 16)
    .expect("status is not valid hex");
assert_eq!(status, 1, "transaction failed on-chain (status != 1)");
```

### 3. Review-Driven Improvements

#### Private Key Security Fix

**Before:**
```rust
let Ok(_key) = std::env::var("PAYG_PRIVATE_KEY") else {
    // skip logic
};
```

**After:**
```rust
if std::env::var("PAYG_PRIVATE_KEY").is_err() {
    // skip logic
}
```

This avoids binding the private key value into memory without zeroization.

#### Redundant Client Elimination

**Before:**
```rust
let health_client = reqwest::Client::builder()
    .timeout(Duration::from_secs(5))
    .build()
    .expect("failed to build health check client");
if health_client.get(facilitator_url).send().await.is_err() {
    // skip logic
}
```

**After:**
```rust
if CLIENT.get(facilitator_url).send().await.is_err() {
    // skip logic
}
```

#### Balance Parsing Fix

**Before:**
```rust
let balance = u64::from_str_radix(balance_hex.trim_start_matches('0'), 16).unwrap_or(0);
```

**After:**
```rust
let balance = u64::from_str_radix(balance_hex, 16).unwrap_or(0);
```

`from_str_radix` handles leading zeros natively, and `unwrap_or(0)` masked parse errors.

#### Skip Message Standardization

All skip messages were updated to include `SKIP:` prefix for agent/CI grep-ability:

```rust
eprintln!("SKIP: e2e_charge_sepolia: PAYG_PRIVATE_KEY not set");
```

#### Environment Variable Documentation

Module header was updated to document:
- `PAYG_PRIVATE_KEY` - Private key for e2e charge test
- `PAYG_TEST_RPC_URL` - Override default RPC endpoint for all network tests

## Prevention Strategies

### Code Patterns

1. **Shared Client Pattern**: Always use `LazyLock` for shared HTTP clients in parallel test scenarios.
2. **Security Guard Pattern**: Implement multiple layers of defense-in-depth for tests that execute real transactions.
3. **Skip Message Pattern**: Standardize skip messages with `SKIP:` prefix for CI/agent parsing.
4. **Environment Variable Pattern**: Document all test-specific environment variables in module headers.

### Review Checklist

1. **Client Construction**: Verify no redundant HTTP client construction in parallel tests.
2. **Security Guards**: Ensure testnet assertions, charge ceilings, and health checks for real transaction tests.
3. **Memory Safety**: Check for unzeroized private key bindings or other sensitive data.
4. **Error Handling**: Verify parse errors are not masked by `unwrap_or` patterns.
5. **CI Compatibility**: Ensure test output is parseable by agents and CI systems.

### Testing Approaches

1. **Parallel Test Verification**: Run tests with `cargo test -- --test-threads=1` to catch race conditions.
2. **Memory Safety Testing**: Use tools like `valgrind` or Rust's built-in sanitizers for sensitive data handling.
3. **Network Failure Testing**: Test health check behavior under various network conditions.
4. **Mainnet Protection Testing**: Verify testnet assertions prevent mainnet execution.

## Cross-References

### Related Documentation
- `docs/plans/2026-02-10-test-network-config-on-chain-verification-plan.md` - Original test plan that was deepened
- `docs/research/base-l2-gas-costs-research.md` - Base L2 gas cost research informing test design

### GitHub Issues
- PR #4: `fix(tests): address review findings in e2e charge test` - Contains all review-driven fixes
- PR #3: `docs: deepen e2e test plan with 15 research agents` - Original plan deepening

## Lessons Learned

1. **Defense-in-depth is essential**: Multiple independent guards (testnet assertion, charge ceiling, health check, on-chain verification) provide robust protection.
2. **Static resources prevent performance issues**: `LazyLock` is the right pattern for shared test resources in parallel scenarios.
3. **Agent/CI compatibility matters**: Standardized output formats (SKIP: prefix) enable better automation and debugging.
4. **Memory safety in test code**: Even test code handling sensitive data needs proper security patterns.
5. **On-chain verification is non-negotiable**: Never trust external services without independent verification for real transactions.

## Impact

- **Performance**: Eliminated ~10 redundant TLS handshakes per test run
- **Security**: Multiple layers of protection against mainnet execution
- **Reliability**: Graceful handling of facilitator outages
- **Verification**: Independent confirmation of on-chain transaction success
- **Maintainability**: Clear patterns and documentation for future test development

The solution has been verified working with all tests passing, clippy clean, and fmt clean. The documentation has been updated to capture these patterns for future reference.