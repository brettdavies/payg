---
title: "CI Feature Matrix Silently Skipped eth-only Backend Tests Due to Additive Rust Features"
date: 2026-02-12
category: build-errors
tags: [rust, ci, feature-flags, github-actions, cargo]
severity: high
component: .github/workflows/ci.yml
symptoms: "eth-only backend code path never tested in CI despite 3-entry feature matrix"
root_cause: "Rust cargo features are additive; --features eth adds to defaults, does not replace them"
pr: 5
---

# CI Feature Matrix Silently Skipped eth-only Backend Tests

## Problem

A Rust workspace has 2 feature-gated payment backends:

- `x402` (USDC via ERC-3009) — **default feature**
- `eth` (direct ETH via alloy-rs) — optional feature

The CI workflow tested 3 matrix entries:

```yaml
matrix:
  features: ["", "--features eth", "--all-features"]
```

This appeared to test: (1) x402-only, (2) eth-only, (3) both. But Rust `cargo` features are **additive** — `--features eth` adds `eth` to the already-enabled default `x402`. So the actual test coverage was:

1. `""` → x402 only
2. `"--features eth"` → x402 + eth (same as #3) **DUPLICATE**
3. `"--all-features"` → x402 + eth

The eth-only code path (including `compile_error!` guards, ETH-specific provider setup, and backend dispatch) was **never tested in isolation**.

## Root Cause

Cargo's `--features` flag is additive by design. It adds features to the set of already-enabled default features. To test a non-default feature in isolation, you must explicitly disable defaults with `--no-default-features`.

## Solution

### 1. Fix the feature matrix

```yaml
# Before (WRONG — entry 2 is duplicate of entry 3):
features: ["", "--features eth", "--all-features"]

# After (CORRECT — all 3 combinations tested):
features: ["", "--no-default-features --features eth", "--all-features"]
```

### 2. Harden expression interpolation

Applied GitHub Actions security best practice: pass matrix values through env vars instead of direct `${{ }}` interpolation to prevent expression injection.

```yaml
# Before (direct interpolation):
- run: cargo test ${{ matrix.features }}

# After (env var indirection):
- name: Run tests
  run: cargo test $FEATURES
  env:
    FEATURES: ${{ matrix.features }}
```

### 3. Merge fmt into lint job

Consolidated the separate `fmt` and `clippy` jobs into a single `lint` job since both use the stable toolchain. Saves one full checkout+toolchain cycle in CI.

## Verification

- All 3 matrix entries now test distinct feature combinations
- CI passes with the corrected matrix
- eth-only compilation verified (catches feature-gated import errors)

## Key Insight

For any Rust project with default features, the CI test matrix must include `--no-default-features` entries to verify non-default features compile and pass tests in isolation. This is especially important when feature flags gate entire backend implementations with `#[cfg(feature = "...")]`.

## Prevention

### Minimum Matrix for N Features

For a workspace with 1 default feature (X) and N optional features:

| Entry | Flags | Result | Purpose |
|-------|-------|--------|---------|
| 1 | (none) | X only | Default path |
| 2 | `--no-default-features --features Y` | Y only | Isolated optional |
| 3 | `--all-features` | X + Y | Interaction test |

**Minimum entries = N + 2** (default + each optional in isolation + all).

### Code Review Heuristic

When reviewing CI for Rust feature matrices:

- **Required**: Every non-default feature must appear in at least one entry with `--no-default-features`
- **Red flag**: `--features X` without `--no-default-features` when X is meant to be tested alone
- **Verify**: Calculate the resolved feature set for each entry — no two entries should be identical

### CI Documentation Pattern

Add a comment block in the CI file explaining the matrix rationale:

```yaml
# Feature matrix strategy:
# - "": Default features only (x402)
# - "--no-default-features --features eth": eth backend in isolation
# - "--all-features": All features together (x402 + eth)
# Each entry produces a distinct set of enabled features.
```

### When This Applies

- Any Rust project with `default = [...]` in `Cargo.toml`
- Features that gate entire modules or backends (`#[cfg(feature = "...")]`)
- Projects with `compile_error!` guards for no-feature builds

## How It Was Caught

Identified by 3 of 6 independent review agents (Performance Oracle, Architecture Strategist, Pattern Recognition) during a parallel multi-agent code review. The redundancy across agents confirmed this was a real issue, not a false positive.

## Cross-References

- [Existing feature-flag solution](../build-errors/rust-workspace-dependency-conflicts-and-feature-flags.md) — covers rand version pinning, feature-flag type leaks, alloy Provider trait imports
- CI workflow: `.github/workflows/ci.yml`
- CLAUDE.md: Feature flag conventions
- MEMORY.md: "Feature flags beat separate crates for <3 known compile-time backends"
