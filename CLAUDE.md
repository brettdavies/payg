# PAYG — Project Conventions

## Structure

Rust workspace with 2 crates:

- `crates/payg` — library crate (payment logic, config, wallet)
- `crates/payg-cli` — binary crate (`payg` CLI)

## Feature Flags

Two payment backends, feature-gated in the lib crate:

- `x402` (default) — ERC-3009 USDC transfers via x402 facilitator
- `eth` — direct ETH transfers via alloy-rs

A `compile_error!` guard fires when neither feature is enabled.

## Build

- **MSRV:** 1.88.0
- **Edition:** 2024
- **Tokio runtime:** `current_thread` (CLI does one sequential operation)

## Test Commands

```sh
cargo test                    # default features (x402)
cargo test --all-features     # both backends
cargo test -- --ignored       # on-chain tests (requires funded wallet)
```

## Config Precedence

CLI flag > env var > config file (`~/.payg/config.toml`) > default.

## Environment Variables

| Variable | Purpose |
|---|---|
| `PAYG_PRIVATE_KEY` | Hex private key (recommended for agents) |
| `PAYG_KEY_PASSWORD` | Keyfile decryption password |
| `PAYG_NETWORK` | Target network: `base`, `base-sepolia` |
| `PAYG_OUTPUT` | Output format: `text`, `json` |
| `PAYG_MAX_CHARGE` | Safety ceiling override (e.g. `5.00 USDC`) |
| `PAYG_RPC_URL` | RPC endpoint override |
| `PAYG_FACILITATOR_URL` | x402 facilitator URL override |

## Networks

| Name | Chain ID | Default |
|---|---|---|
| `base` | 8453 | No (mainnet, real funds) |
| `base-sepolia` | 84532 | Yes (testnet) |

## Key Conventions

- `PaygError::Http(String)` not `Http(reqwest::Error)` — avoids feature-flag leak
- Pin `rand = "0.8"` — alloy-rs compatibility
- ETH provider uses `.connect(&str)` not `.connect_http(Url)` — avoids reqwest leak
- Use individual alloy sub-crates, not the `alloy` facade crate
- x402 facilitator pays gas; signer only signs ERC-3009 auth locally

## Versioning

- **Scheme:** Semantic versioning (major.minor.patch)
- **Tool:** [release-plz](https://release-plz.dev/) — automated via GitHub Actions
- **Both crates version in lockstep** via `version_group` in `release-plz.toml`
- **Changelog:** Single root `CHANGELOG.md`, auto-generated from Conventional Commits
- **Release flow:** merge to `main` → release-plz opens Release PR → merge PR → git tag + GitHub release
- **Not published to crates.io yet** — `git_only = true` in `release-plz.toml`
- **Merge strategy for Release PRs:** Use standard merge commit (not squash/rebase) — squash creates a race condition in release-plz's detection
