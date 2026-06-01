---
name: payg
binary: payg
description: Pay-as-you-go crypto micropayments for CLI tools on Base L2. Per-invocation USDC (via x402) or direct ETH transfers from a Rust workspace shipping both a library (`payg`) and a binary (`payg`).
homepage: https://github.com/brettdavies/payg
repository: https://github.com/brettdavies/payg
---

# AGENTS.md

## Running payg

```bash
# Initialize a wallet (interactive — prompts for keyfile password)
payg init

# Inspect the current wallet address (env- or keyfile-loaded)
payg address

# Check balance (USDC if x402 backend, ETH if eth backend)
payg balance

# Charge a micropayment from the wallet (dry-run by default v1)
payg charge --amount 0.001 --to 0xRECIPIENT --dry-run

# JSON output for parsing
PAYG_OUTPUT=json payg balance

# Override network (mainnet vs testnet)
PAYG_NETWORK=base-sepolia payg balance       # testnet, no real funds
PAYG_NETWORK=base payg balance               # mainnet, real funds
```

Bare `payg` (no arguments) prints help and exits.

## Structure

Rust workspace with 2 crates:

- `crates/payg` — library crate (payment logic, config, wallet).
- `crates/payg-cli` — binary crate (`payg` CLI).

The library is consumable from downstream Rust crates; the binary is one consumer among potentially several. Other
agent-callable consumers shell out to `payg charge` regardless of host language.

## Feature flags

Two payment backends, feature-gated in the lib crate. A `compile_error!` guard fires when neither feature is enabled.

| Feature | Default | Backend              | Mechanism                                                                         |
| ------- | ------- | -------------------- | --------------------------------------------------------------------------------- |
| `x402`  | yes     | USDC via x402        | ERC-3009 authorization signed locally; facilitator settles on-chain and pays gas. |
| `eth`   | no      | Direct ETH via alloy | Signer submits a native ETH transfer; signer pays gas.                            |

The `Backend` enum (`crates/payg/src/backend.rs`) dispatches at runtime via `match`. No `dyn`, no `async_trait`, no heap
allocation. Backend selection is implicit per crate features at compile time.

## Wallet paths

Two ways to load a wallet, selected by what's available in the environment.

| Path           | When used                                                   | Latency    | Recommended for |
| -------------- | ----------------------------------------------------------- | ---------- | --------------- |
| Env var        | `PAYG_PRIVATE_KEY=<hex>` set in the environment             | 10–30 ms   | Agents, CI      |
| Keyfile (JSON) | `~/.payg/keyfile.json` + `PAYG_KEY_PASSWORD` for decryption | 200–500 ms | Humans, dev     |

`crates/payg/src/wallet.rs` is sync (no async on the wallet load path). Keyfile decryption uses scrypt — the latency
floor is the KDF, not I/O. Env var is the agent-recommended path because scrypt's ~210–530ms per invocation adds up
across automated calls.

## Networks

| Name           | Chain ID | Default | Purpose              |
| -------------- | -------- | ------- | -------------------- |
| `base-sepolia` | 84532    | yes     | Testnet              |
| `base`         | 8453     | no      | Mainnet (real funds) |

Default is testnet so accidental `payg charge` invocations during development do not move real money. Explicit
`PAYG_NETWORK=base` is required for mainnet.

## Config precedence

Standard Unix precedence applies throughout:

```text
CLI flag > env var > config file (~/.payg/config.toml) > default
```

## Environment variables

| Variable               | Purpose                                    |
| ---------------------- | ------------------------------------------ |
| `PAYG_PRIVATE_KEY`     | Hex private key (recommended for agents)   |
| `PAYG_KEY_PASSWORD`    | Keyfile decryption password                |
| `PAYG_NETWORK`         | Target network: `base`, `base-sepolia`     |
| `PAYG_OUTPUT`          | Output format: `text`, `json`              |
| `PAYG_MAX_CHARGE`      | Safety ceiling override (e.g. `5.00 USDC`) |
| `PAYG_RPC_URL`         | RPC endpoint override                      |
| `PAYG_FACILITATOR_URL` | x402 facilitator URL override              |

## Output formats

`PAYG_OUTPUT` and the `--output` flag drive two formats:

- `text` (default): human-readable output.
- `json`: pretty-printed JSON envelope.

Agent-callable consumers should set `PAYG_OUTPUT=json` to get parseable output without scraping prose.

## Architecture

- `crates/payg/src/backend.rs` — `Backend` enum (`X402(...)` / `Eth(...)`); `match` dispatch.
- `crates/payg/src/backend/x402.rs` — x402-chain-eip155 ERC-3009 signing + facilitator POST.
- `crates/payg/src/backend/eth.rs` — alloy-rs direct ETH transfer (`.connect(&str)` to avoid `reqwest` leak).
- `crates/payg/src/wallet.rs` — sync wallet loading: env var path and scrypt-decrypted keyfile path.
- `crates/payg/src/charge.rs` — `ChargeRequest` → `Receipt` (tx_hash); no `Token` enum (backend is the disambiguator),
  no chain_id (network selects it).
- `crates/payg/src/config.rs` — TOML config loader; CLI > env > config > default precedence.
- `crates/payg/src/network.rs` — network registry: chain id, default RPC, default facilitator URL.
- `crates/payg/src/pricing.rs` — amount normalization across token decimals.
- `crates/payg/src/error.rs` — `PaygError`. `Http(String)` not `Http(reqwest::Error)` — avoids a `reqwest` feature-flag
  leak across the public error surface.
- `crates/payg-cli/src/cli.rs` — clap-based CLI definition.
- `crates/payg-cli/src/commands/{address,balance,charge,init}.rs` — per-command handlers.

## Key conventions

- `PaygError::Http(String)` not `Http(reqwest::Error)` — avoids feature-flag leak across the public error type.
- Pin `rand = "0.8"` — alloy-rs compatibility (alloy uses 0.8 traits; mixing 0.9 produces trait mismatches).
- ETH provider uses `.connect(&str)` not `.connect_http(reqwest::Url)` — avoids reqwest leak through the alloy facade.
- Use individual `alloy-*` sub-crates (`alloy-provider`, `alloy-signer-local`, `alloy-primitives`,
  `alloy-rpc-types-eth`), not the `alloy` umbrella crate.
- x402 facilitator pays gas; the signer only signs ERC-3009 auth locally. No ETH balance required for x402 charges.
- `current_thread` tokio runtime in the CLI — one sequential operation per invocation; multi-thread overhead is
  unnecessary.

## Versioning

- **Scheme:** Semantic versioning (major.minor.patch).
- **Tool:** [release-plz](https://release-plz.dev/) — automated via GitHub Actions.
- **Both crates version in lockstep** via `version_group` in `release-plz.toml`.
- **Changelog:** Single root `CHANGELOG.md`, auto-generated from Conventional Commits by release-plz (no `cliff.toml`,
  no `generate-changelog.sh`).
- **Release flow:** merge to `main` → release-plz opens Release PR → merge PR → git tag + GitHub release.
- **Not published to crates.io yet** — `git_only = true` in `release-plz.toml`.
- **Merge strategy:** Standard merge commit to `main` (not squash/rebase). Squash creates a race condition in
  release-plz where unreviewed commits can slip into a release. Enforced by ruleset.

## Quality bar

- Clippy clean, edition 2024 (`cargo clippy --all-targets --all-features -- -D warnings`).
- Formatted with rustfmt (`cargo fmt --all -- --check`); style edition pinned in `rustfmt.toml`.
- All three feature-matrix configurations pass `cargo test` (default, `--no-default-features --features eth`,
  `--all-features`).
- MSRV verified (`cargo +1.88.0 check --all-features`); MSRV is declared in workspace `Cargo.toml`.
- `cargo deny check` passes (advisories, licenses, bans, sources).
- `cargo doc --no-deps --all-features` clean with `RUSTDOCFLAGS=-D warnings`.

The pinned toolchain (`rust-toolchain.toml`) is the supply-chain anchor. Rustup verifies component SHA256s from the
distribution manifest; the pin is effectively a SHA pin. Toolchain bumps land via reviewed PR after ≥7-day quarantine.

## Testing

```bash
cargo test                                          # default features (x402)
cargo test --no-default-features --features eth     # eth-only backend
cargo test --all-features                           # both backends
cargo test -- --ignored                             # on-chain tests (requires funded wallet)
scripts/hooks/pre-push                              # full local CI mirror
```

The pre-push hook mirrors CI 1:1 plus a Windows cross-compilation check. Run it before pushing if `core.hooksPath =
scripts/hooks` is not set locally.

## Git hooks

Project-local hooks live in `scripts/hooks/`. After cloning, activate them:

```sh
git config core.hooksPath scripts/hooks
```

**Pre-commit** enforces:

- No direct commits to `main` (use a feature branch + PR).
- Commit signing must be enabled (`commit.gpgsign = true`).

**Pre-push** mirrors the CI pipeline locally:

- `cargo fmt --all -- --check`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo test` across the three-feature matrix (default, eth-only, all-features)
- `cargo +<msrv> check --all-features` (requires `rustup toolchain install <msrv>`)
- `cargo doc --no-deps --all-features` with `RUSTDOCFLAGS=-D warnings`
- `cargo deny check` (skipped if cargo-deny not installed)
- `shellcheck --severity=warning` on tracked shell scripts (skipped if not installed)
- Windows libc grep (catches unconditional unix-only API usage outside `#[cfg(unix)]`)
- Windows cross-clippy against `x86_64-pc-windows-gnu` (skipped if mingw-w64 + the target are not installed)

**Agent/CI signing setup:**

```sh
git config user.signingkey <YOUR_KEY_ID>
git config gpg.format ssh    # or "openpgp" for GPG keys
git config commit.gpgsign true
```

## PR template

A pull request template lives in `.github/pull_request_template.md`. GitHub auto-populates it when opening a PR. The PR
title should follow Conventional Commits: `type(scope): description`.

## Branch workflow

- `dev` — integration branch; PRs target here.
- `main` — protected; receives merge commits from `dev` only via PR.
- Feature branches — branch from `dev`, PR back to `dev`.

**NEVER commit directly to `main`.** All work happens on feature branches off `dev`. The pre-commit hook blocks direct
commits to `main`, and the GitHub ruleset requires PRs with passing CI. The only path to `main` is `dev` → `main` via
merge PR.

Planning-only docs (`docs/brainstorms/`, `docs/ideation/`, `docs/plans/`, `docs/research/`, `docs/reviews/`,
`docs/solutions/`) may be committed directly to `dev` without a feature branch.

## Releasing

See [`RELEASES.md`](RELEASES.md) for the operational runbook, [`RELEASES-PREFLIGHT.md`](RELEASES-PREFLIGHT.md) for the
pre-cut go/no-go checklist, and [`RELEASES-RATIONALE.md`](RELEASES-RATIONALE.md) for the why behind every rule.
