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

| Variable               | Purpose                                    |
| ---------------------- | ------------------------------------------ |
| `PAYG_PRIVATE_KEY`     | Hex private key (recommended for agents)   |
| `PAYG_KEY_PASSWORD`    | Keyfile decryption password                |
| `PAYG_NETWORK`         | Target network: `base`, `base-sepolia`     |
| `PAYG_OUTPUT`          | Output format: `text`, `json`              |
| `PAYG_MAX_CHARGE`      | Safety ceiling override (e.g. `5.00 USDC`) |
| `PAYG_RPC_URL`         | RPC endpoint override                      |
| `PAYG_FACILITATOR_URL` | x402 facilitator URL override              |

## Networks

| Name           | Chain ID | Default                  |
| -------------- | -------- | ------------------------ |
| `base`         | 8453     | No (mainnet, real funds) |
| `base-sepolia` | 84532    | Yes (testnet)            |

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
- **Merge strategy:** Standard merge commit to `main` (not squash/rebase) — squash creates a race condition in
  release-plz where unreviewed commits can slip into a release. Enforced by ruleset.

## Git Hooks

Project-local hooks live in `.githooks/`. After cloning, activate them:

```sh
git config core.hooksPath .githooks
```

**Pre-commit hook** enforces:

- No direct commits to `main` (use a feature branch + PR)
- Commit signing must be enabled (`commit.gpgsign = true`)

**Agent/CI signing setup:**

```sh
git config user.signingkey <YOUR_KEY_ID>
git config gpg.format ssh    # or "openpgp" for GPG keys
git config commit.gpgsign true
```

## PR Template

A pull request template lives in `.github/pull_request_template.md`. GitHub auto-populates it when opening a PR. The PR
title should follow Conventional Commits: `type(scope): description`.

## Branch Workflow

- `dev` — integration branch; PRs target here
- `main` — protected; receives merge commits from `dev` only via PR
- Feature branches — branch from `dev`, PR back to `dev`

**NEVER commit directly to `main`.** All work happens on feature branches off `dev`. The pre-commit hook blocks direct
commits to `main`, and the GitHub ruleset requires PRs with passing CI. The only path to `main` is `dev` → `main` via
merge PR.
