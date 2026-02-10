---
title: "feat: PAYG Crypto Micropayment System for CLI Tools"
type: feat
date: 2026-02-09
deepened: 2026-02-10
---

# PAYG: Pay-As-You-Go Crypto Micropayment System for CLI Tools

> **Enhancement Summary** (added by `/deepen-plan` on 2026-02-10)
>
> This plan was enhanced with findings from 8 parallel research/review agents covering: x402 EIP-3009 signing API, alloy-rs transaction patterns, security analysis, performance profiling, architecture review, code simplicity review, pattern recognition, and agent-native architecture analysis. Key changes:
>
> 1. **Replace `PaymentBackend` trait with `Backend` enum** — eliminates `async_trait` dependency, heap allocation per charge, and invalid states (enum is exhaustive at compile time)
> 2. **Drop `#[async_trait]`** — native `async fn` in traits stable since Rust 1.75, well within MSRV 1.88.0
> 3. **Simplify `ChargeRequest`/`ChargeReceipt`** — remove `Token` enum (redundant with backend selection), remove `chain_id` (hardcoded Base for v1), receipt = tx_hash only
> 4. **Fix `reqwest::Error` feature-flag leak** — `Http(#[from] reqwest::Error)` fails to compile with only `eth` feature; use string wrapping
> 5. **Add `serde_json` to explicit dependencies** — currently only transitive, will break
> 6. **Use `current_thread` tokio runtime** — CLI does one sequential operation; multi-thread spawns unnecessary worker threads
> 7. **Document `PAYG_PRIVATE_KEY` as the agent-recommended path** — avoids 200-500ms scrypt keyfile decryption per invocation
> 8. **Add `facilitator_url` to consumer config** — single point of failure mitigation
> 9. **Add `compile_error!` guard** — when no backend feature is enabled
> 10. **Add v1 agent-native commands** — `payg balance`, `payg address`, `--dry-run`, structured JSON errors with exit code 402

## Overview

Build a drop-in pay-as-you-go billing system that developers can integrate into CLI tools. Payments are peer-to-peer crypto micropayments on Base L2, using x402 (USDC) as the default backend and direct ETH transfer as an alternative. Distributed as a Rust crate (`cargo add payg`) and a standalone binary (`brew install payg`). Targets agentic workflows where autonomous agents pay fractions of a cent per tool invocation.

> **Research Insight — Agent-Native Architecture**: PAYG's primary consumer is AI agents operating autonomously. Design every command and output format with machine-first consumption in mind. Human-friendly output is a secondary formatting layer, not the default. Exit codes, structured JSON, and introspection commands (`balance`, `address`, `context`) are critical for agent workflows.

## Problem Statement

CLI tool developers have no simple way to monetize per-invocation usage. Subscription models punish casual users and agents. Existing payment infrastructure (Stripe, etc.) is too heavyweight for $0.001 charges and doesn't work for headless/agent consumers. The x402 protocol solves the HTTP API case but doesn't address CLI tools. PAYG bridges this gap.

## Proposed Solution

A Rust workspace with 2 crates (`payg` lib + `payg-cli` bin) implementing a three-tier architecture:

- **Primitive**: `payg::charge(amount, recipient)` — the raw payment operation
- **Patterns**: CLI wrapping, `payg.toml` config, subprocess invocation
- **Policies**: Safety ceiling ($1.00 max per charge)

Two payment backends from v1: x402 (USDC, zero client gas) using `x402-chain-eip155` for EIP-3009 signing and direct HTTP calls to the facilitator, and direct ETH transfer via `alloy-rs`. Feature-gated in a single crate. Simple wallet approach: local encrypted keyfile + env var.

## Technical Approach

### Architecture

```
Developer DX Layer (PAYG)
  |- payg.toml + CLI binary (cross-language)
  |- payg::charge() for Rust (library)
  |- Wallet: local keyfile + env var
  '- Safety ceiling, config
      |
Payment Backend Layer (feature-gated)
  |- x402 backend (Base/USDC -- default, zero client gas)
  |- Direct ETH backend (Base/ETH via alloy-rs)
  '- Lightning backend (sats -- future)
```

> **Research Insight — Architecture Review**: The 2-crate split is sound. The `backend.rs` + `backend/` directory layout works natively in edition 2024 (stabilized in Rust 1.85) where `backend.rs` can coexist with `backend/` as a directory. Confirm `edition = "2024"` is set in both crate Cargo.toml files to enable this module resolution.

### Crate Structure

```
payg/
  Cargo.toml                    # workspace root
  crates/
    payg/                       # Core library: charge(), config, wallet, backends
      Cargo.toml                # features = ["x402" (default), "eth"]
      src/
        lib.rs                  # pub fn charge(), re-exports
        error.rs                # PaygError enum
        backend.rs              # PaymentBackend trait
        backend/
          x402.rs               # X402Backend (feature = "x402")
          eth.rs                # EthBackend (feature = "eth")
        wallet.rs               # load_wallet() function
        config.rs               # Config structs (payg.toml, ~/.payg/config.toml)
        pricing.rs              # Price parsing ("0.001 USDC")
        charge.rs               # ChargeRequest, ChargeReceipt, charge() orchestration
    payg-cli/                   # CLI binary
      Cargo.toml
      src/
        main.rs
        cli.rs                  # clap definitions
        commands/
          mod.rs
          init.rs               # payg init
          charge.rs             # payg charge
  examples/
    simple-cli/                 # Rust CLI tool using payg::charge()
    python-tool/                # Python script calling `payg charge`
  docs/
```

### Key Dependencies

| Crate | Version | Purpose |
|---|---|---|
| `x402-types` | 1.0 | x402 protocol types |
| `x402-chain-eip155` | 1.0 | EIP-3009 signing (`sign_erc3009_authorization()`) |
| `alloy-provider` | 1.4 | ETH provider + transaction sending (ETH backend only) |
| `alloy-signer-local` | 1.4 | Local keystore signing (`PrivateKeySigner`) |
| `alloy-primitives` | 1.4 | `Address`, `U256`, `FixedBytes` types |
| `alloy-rpc-types-eth` | 1.4 | `TransactionRequest` type (ETH backend only) |
| `clap` | 4.5 | CLI framework (derive API) |
| `tokio` | 1.35 | Async runtime |
| `reqwest` | 0.13 | HTTP client (facilitator API calls) |
| `serde` + `serde_json` + `toml` | 1.0 / 1.0 / 0.8 | Config + payload serialization |
| `eth-keystore` | 0.14 | Web3 V3 encrypted keystore |
| `thiserror` | 2.0 | Error types |
| `tracing` | 0.1 | Logging/diagnostics |

> **Research Insight — Performance**: Do NOT use the `alloy` facade crate with default features. It pulls in `alloy-contract`, `alloy-ens`, `alloy-dyn-abi`, `alloy-json-abi` and others not needed. Depend on individual sub-crates: `alloy-provider`, `alloy-signer-local`, `alloy-primitives`, `alloy-rpc-types-eth`. This reduces binary size by ~5-10MB and avoids duplicate crypto library linkage. Do NOT enable the `secp256k1` feature on `alloy-signer-local` — `k256` (pure Rust) is sufficient and simplifies cross-compilation for cargo-dist targets.
>
> **Research Insight — Pattern Recognition**: Pin `eth-keystore` to 0.14 (not `*`). Version wildcards are a red flag even in plans. Also: `serde_json` is listed in the error enum (`Json(#[from] serde_json::Error)`) but was missing from the dependency table — now added.

Note: x402-rs uses edition 2024 with MSRV 1.88.0. PAYG must match this MSRV since x402-rs is a core dependency.

### Key Design Decisions (Post-Review)

**x402 integration approach**: The `x402-reqwest` crate is HTTP middleware for intercepting 402 responses — it is NOT a direct payment API. PAYG's x402 backend must instead:
1. Use `x402-chain-eip155` to call `sign_erc3009_authorization()` directly
2. Construct the payment payload manually
3. Make a direct HTTP POST to the facilitator's `/settle` endpoint at `https://facilitator.x402.rs`

Reference: `~/github-stars/x402-rs/x402-rs/crates/chains/x402-chain-eip155/src/`

> **Research Insight — x402 API Details**: The `sign_erc3009_authorization()` function (in `x402-chain-eip155/src/v1_eip155_exact/client.rs:144-203`) takes a `SignerLike` + `Eip3009SigningParams` and returns an `ExactEvmPayload`. The `SignerLike` trait requires `fn address() -> Address` + `async fn sign_hash(&FixedBytes<32>) -> Result<Signature>`. `PrivateKeySigner` satisfies this. The settle endpoint expects a JSON `SettleRequest` containing `x402Version`, `paymentPayload` (with signature + authorization data), and `paymentRequirements`. This is moderately complex nested JSON construction — factor into implementation effort.
>
> **Research Insight — x402-rs Issue #26 (CRITICAL)**: x402-rs had an "invalid signature" bug on OP-stack chains (including Base). The root cause was gas estimation using `pending` block instead of `latest`. Fix: the facilitator must use `estimateGas(...).block(latest)`. This is a facilitator-side issue (not client-side), but be aware of it when debugging failed settlements. If running a self-hosted facilitator, apply this fix.

**No daemon for v1**: The charge operation involves 500ms-2000ms of network I/O (facilitator API + on-chain settlement). Non-Rust tools shell out to `payg charge`. Daemon can be added in v1.1 if subprocess latency becomes a real issue.

> **Research Insight — Performance**: The plan's original 40-90ms subprocess overhead estimate is too low. Actual cold start breakdown: process fork+exec (2-5ms) + binary load (5-15ms) + tokio runtime (3-8ms) + config parse (1-3ms) + **keyfile scrypt decryption (200-500ms)**. Total non-network overhead: 210-530ms for keyfile path, 10-30ms for env var path. The scrypt decryption dominates. Mitigation: (1) document `PAYG_PRIVATE_KEY` env var as the recommended agent path, (2) use lighter scrypt params (N=8192 vs N=262144) for PAYG-generated keyfiles — reduces decryption from ~300ms to ~10ms (acceptable for micropayment floats, not cold storage).

**No proc macros for v1**: `payg::charge()` is a one-line call. The `#[payg(cost = "0.001")]` macro saves one line per function but adds `syn`+`quote` compile-time deps, a mandatory separate crate, async/sync detection edge cases, and `trybuild` test infrastructure. Ship v1 with the direct call; add macros in v1.1 when the charge-then-execute pattern is validated by real users.

**No HTTP middleware**: x402-axum already provides HTTP 402 middleware. PAYG is CLI-native. Rebuilding HTTP middleware in PAYG adds no value over using x402-axum directly.

**Feature flags over separate crates**: Two known backends, both written by the same developer, both known at compile time. Feature flags (`x402` default, `eth` optional) are the right tool. The "feature flag complexity" concern from the brainstorm applies to mature multi-contributor projects, not a v1 with two flags.

**Simple wallet for v1**: One `load_wallet()` function, not a trait with four implementations. Checks `PAYG_PRIVATE_KEY` env var first, falls back to local encrypted keyfile at `~/.payg/keyfile.enc`. When real users ask for CDP v2 or 1Password integration, add the trait and providers.

### Implementation Phases

#### Phase 1: Core Library + x402 Backend

Scaffold the workspace, implement `payg` lib with core types, config, wallet, and the x402 payment backend.

**Tasks:**

- [ ] Initialize git repo with `.gitignore` (target/, .env, *.enc, ~/.payg/) `[.gitignore]`
- [ ] Create root `Cargo.toml` with workspace definition `[Cargo.toml]`
- [ ] Create `rustfmt.toml` and `clippy.toml` `[rustfmt.toml, clippy.toml]`
- [ ] Scaffold `payg` lib crate with feature flags `[crates/payg/Cargo.toml]`
  ```toml
  [features]
  default = ["x402"]
  x402 = ["dep:x402-types", "dep:x402-chain-eip155", "dep:reqwest"]
  eth = ["dep:alloy-provider", "dep:alloy-rpc-types-eth", "dep:alloy-network"]
  ```
  Add compile guard in `lib.rs`:
  ```rust
  #[cfg(not(any(feature = "x402", feature = "eth")))]
  compile_error!("At least one payment backend feature must be enabled: 'x402' or 'eth'");
  ```
  Add release profile:
  ```toml
  [profile.release]
  strip = true
  lto = "thin"
  codegen-units = 1
  opt-level = "s"  # Optimize for size — CLI startup matters more than throughput
  ```
- [ ] Scaffold `payg-cli` bin crate `[crates/payg-cli/Cargo.toml]`
- [ ] Implement `PaygError` enum `[crates/payg/src/error.rs]`
  ```rust
  #[derive(Debug, thiserror::Error)]
  pub enum PaygError {
      #[error("no wallet configured")]
      NoWallet,
      #[error("charge amount {amount} exceeds safety ceiling {ceiling}")]
      ExceedsSafetyCeiling { amount: String, ceiling: String },
      #[error("payment failed: {0}")]
      PaymentFailed(String),
      #[error("config error: {0}")]
      ConfigError(String),
      #[error("wallet error: {0}")]
      WalletError(String),
      #[error(transparent)]
      Io(#[from] std::io::Error),
      #[error("http error: {0}")]
      Http(String),  // String, not #[from] reqwest::Error — reqwest is feature-gated
      #[error("json error: {0}")]
      Json(#[from] serde_json::Error),
  }
  ```
  > **Research Insight — Pattern Recognition**: The original `Http(#[from] reqwest::Error)` would fail to compile when only the `eth` feature is enabled (reqwest is gated behind `x402`). Using `Http(String)` with `.map_err(|e| PaygError::Http(e.to_string()))` at call sites avoids the feature-flag leak. This also prevents callers from matching on reqwest internals — a good boundary.
- [ ] Implement `Backend` enum with static dispatch `[crates/payg/src/backend.rs]`
  ```rust
  pub enum Backend {
      #[cfg(feature = "x402")]
      X402(X402Backend),
      #[cfg(feature = "eth")]
      Eth(EthBackend),
  }

  impl Backend {
      pub async fn charge(&self, request: &ChargeRequest) -> Result<ChargeReceipt, PaygError> {
          match self {
              #[cfg(feature = "x402")]
              Self::X402(b) => b.charge(request).await,
              #[cfg(feature = "eth")]
              Self::Eth(b) => b.charge(request).await,
          }
      }
  }
  ```
  > **Research Insight — Simplicity + Architecture**: All reviews converged on this: replace the `PaymentBackend` trait with a `Backend` enum. Two backends, same author, compile-time feature-gated — a trait is the wrong abstraction. The enum eliminates: `async_trait` dependency, `Box::pin` heap allocation per charge, `dyn` dispatch, `Send + Sync` bounds, `name()` method (use tracing spans instead). The compiler enforces exhaustive match arms if a backend is added. If a third-party backend is ever needed, refactoring an enum to a trait is trivial — the reverse is not.
- [ ] Implement `ChargeRequest` and `ChargeReceipt` types `[crates/payg/src/charge.rs]`
  ```rust
  #[derive(Debug, Clone)]
  pub struct ChargeRequest {
      pub amount: U256,
      pub recipient: Address,
  }

  #[derive(Debug, Clone, Serialize)]
  pub struct ChargeReceipt {
      pub tx_hash: String,
  }
  ```
  > **Research Insight — Simplicity Review**: `Token` enum removed — it was redundant with backend selection. The x402 backend always uses USDC, the ETH backend always uses ETH. Having `Token` as a field creates an invalid state (Token::Eth + x402 backend). `chain_id` removed — Base L2 (8453) is the only target for v1, hardcoded as a constant. Receipt fields `amount`/`token`/`chain_id` removed — the caller already knows these from the request. The CLI serialization layer can add them back for JSON output.
  >
  > **Research Insight — Architecture Review**: The public `charge()` API should accept human-readable strings and parse internally: `payg::charge("0.001 USDC", "0xABC...").await?`. `ChargeRequest` is the parsed internal representation. Add `Token::decimals()` metadata (USDC=6, ETH=18) for correct U256 conversion. The USDC contract address on Base (`0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913`) should be a constant.
- [ ] Implement `load_wallet()` function `[crates/payg/src/wallet.rs]`
  ```rust
  pub fn load_wallet(config: &ConsumerConfig) -> Result<PrivateKeySigner, PaygError> {
      // 1. Check PAYG_PRIVATE_KEY env var (fast path — recommended for agents)
      if let Ok(key) = std::env::var("PAYG_PRIVATE_KEY") {
          tracing::warn!("Using raw private key from env var (testing/agent use only)");
          return Ok(key.parse().map_err(|e| PaygError::WalletError(format!("{e}")))?);
      }
      // 2. Load from encrypted keyfile (200-500ms scrypt decryption)
      let keyfile = config.keyfile_path();
      let password = prompt_or_env_password()?;
      Ok(PrivateKeySigner::decrypt_keystore(keyfile, password)
          .map_err(|e| PaygError::WalletError(format!("{e}")))?)
  }
  ```
  > **Research Insight — Performance + Security**: Function is now sync (was `async` but all operations are synchronous — `decrypt_keystore` uses CPU-bound scrypt, not I/O). The scrypt KDF in eth-keystore uses standard Web3 params (N=262144, r=8, p=1) = ~200-500ms per decrypt. For agent workflows doing 100+ charges/day, this adds 20-50s of pure CPU overhead. Document `PAYG_PRIVATE_KEY` as the recommended agent path. Consider using lighter scrypt params (N=8192) for `payg init`-generated keyfiles — reduces to ~10ms, acceptable for micropayment floats.
  >
  > **Research Insight — Security Review**: The keyfile password source (`prompt_or_env_password()`) should check `PAYG_KEY_PASSWORD` env var first, then fall back to interactive prompt. For headless/agent use, the env var path must work without a TTY. Never log/trace the password or private key, even at debug level. Validate the env-var private key format before parsing to provide clear error messages.
- [ ] Implement config structs with serde `[crates/payg/src/config.rs]`
  - `ProjectConfig` (payg.toml):
    ```toml
    recipient = "0x1234...abcd"
    default_price = "0.001 USDC"
    ```
  - `ConsumerConfig` (~/.payg/config.toml):
    ```toml
    keyfile = "~/.payg/keyfile.enc"
    max_charge = "1.00 USDC"
    facilitator_url = "https://facilitator.x402.rs"  # configurable SPOF mitigation
    ```
  - Config discovery: look for `payg.toml` in CWD (no directory walking for v1), load `~/.payg/config.toml` from home dir
  - ENV var overrides: `PAYG_PRIVATE_KEY`, `PAYG_MAX_CHARGE`
  > **Research Insight — Simplicity Review**: Per-command `[commands]` pricing removed from v1 — adds config parsing complexity, extra `--command` CLI flag, and edge cases. For v1, `default_price` covers the common case. Advanced users pass explicit amounts: `payg charge 0.01`. Add `[commands]` in v1.1 when validated by real usage. `PAYG_CHAIN` env var removed — Base is hardcoded for v1. Directory walking for `payg.toml` removed — the tool dev puts it in the project root; the CLI looks in CWD. If not found, use `--config <path>` or explicit CLI args.
  >
  > **Research Insight — Architecture Review**: `facilitator_url` added to ConsumerConfig — the x402 facilitator at `https://facilitator.x402.rs` is a single point of failure. Making it configurable allows self-hosted facilitators without code changes. Also: config precedence should be explicit: ENV > consumer config > project config > defaults. Bound the config file read to the home directory or filesystem root to prevent picking up a `payg.toml` placed in `/` or `/tmp`.
- [ ] Implement price parsing: "0.001 USDC", "0.01 ETH", "free" `[crates/payg/src/pricing.rs]`
- [ ] Implement `charge()` orchestration function `[crates/payg/src/charge.rs]`
  - Load config → check safety ceiling → select backend by token → call backend.charge()
- [ ] Implement `X402Backend` (feature-gated) `[crates/payg/src/backend/x402.rs]`
  - Uses `x402-chain-eip155` for `sign_erc3009_authorization()`
  - Constructs payment payload with signed EIP-3009 authorization
  - Makes direct HTTP POST to facilitator `/settle` at configurable URL (default: `https://facilitator.x402.rs`)
  - Does NOT depend on `x402-reqwest` (that's HTTP middleware, not a payment API)
  - Reference: `~/github-stars/x402-rs/x402-rs/crates/chains/x402-chain-eip155/src/`
  > **Research Insight — x402 API Concrete Implementation**:
  > ```rust
  > // 1. Create signer that satisfies SignerLike trait
  > let signer = load_wallet(&config)?;
  >
  > // 2. Build signing params
  > let params = Eip3009SigningParams {
  >     token_name: "USD Coin".to_string(),
  >     chain_id: 8453,  // Base mainnet
  >     verifying_contract: "0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913".parse()?,
  >     from: signer.address(),
  >     to: recipient,
  >     value: amount,
  >     valid_after: U256::ZERO,
  >     valid_before: U256::from(timestamp + 3600),  // 1 hour window
  >     nonce: random_nonce(),
  > };
  >
  > // 3. Sign the EIP-3009 authorization
  > let payload = sign_erc3009_authorization(&signer, params).await?;
  >
  > // 4. POST to facilitator /settle
  > let settle_request = json!({
  >     "x402Version": 2,
  >     "paymentPayload": { /* ExactEvmPayload serialized */ },
  >     "paymentRequirements": { /* price tag requirements */ }
  > });
  > let response = reqwest::Client::new()
  >     .post(format!("{}/settle", config.facilitator_url))
  >     .json(&settle_request)
  >     .send().await?;
  > ```
  > Log `tx_hash` before awaiting confirmation. If the settle call times out but payment was submitted on-chain, include the tx_hash in the error for manual verification.
- [ ] Implement `EthBackend` (feature-gated) `[crates/payg/src/backend/eth.rs]`
  - Connects to Base L2 via RPC URL (default: `https://mainnet.base.org`)
  - Builds `TransactionRequest`, sends via alloy provider, awaits receipt
  - Reference: `~/github-stars/alloy-rs/alloy/` examples
  > **Research Insight — alloy-rs Concrete Implementation**:
  > ```rust
  > let signer = load_wallet(&config)?;
  > let provider = ProviderBuilder::new()
  >     .wallet(signer)
  >     .connect_http("https://mainnet.base.org".parse()?);
  >
  > let tx = TransactionRequest::default()
  >     .with_to(recipient)
  >     .with_value(amount);
  >
  > // Gas, nonce, chain_id auto-filled by provider fillers
  > let pending = provider.send_transaction(tx).await?;
  > let receipt = pending
  >     .with_timeout(Some(Duration::from_secs(60)))
  >     .get_receipt().await?;
  > ```
  > alloy's `ProviderBuilder` with `.wallet()` auto-fills gas (EIP-1559 `max_fee_per_gas` + `max_priority_fee_per_gas` via `GasFiller`), nonce (via `CachedNonceManager`), and chain_id. No manual nonce or gas management needed for v1 single-transaction use. For v1.1 concurrent agents, a nonce mutex will be needed.
- [ ] Write unit tests for config parsing, price parsing, safety ceiling logic `[crates/payg/src/tests/]`
- [ ] Write integration tests for x402 payment on Base Sepolia `[crates/payg/tests/x402_integration.rs]`
- [ ] Write integration tests for ETH payment on Base Sepolia `[crates/payg/tests/eth_integration.rs]`
- [ ] Set up GitHub Actions CI: test, clippy, fmt, MSRV check `[.github/workflows/ci.yml]`

**Success criteria:** `cargo build --workspace` succeeds. Both backends can execute real payments on Base Sepolia. `payg::charge()` works end-to-end.

#### Phase 2: CLI Binary

Implement the CLI binary with `init` and `charge` subcommands.

**Tasks:**

- [ ] Define clap CLI struct with subcommands `[crates/payg-cli/src/cli.rs]`
  - Global flags: `--output json|text`
  - Subcommands: `init`, `charge`, `balance`, `address`
  - Use `#[tokio::main(flavor = "current_thread")]` in `main.rs` — CLI does one sequential operation, no need for multi-thread runtime (saves 3-8ms startup + avoids spawning worker threads)
  > **Research Insight — Agent-Native Architecture**: Add these v1 commands for agent consumers:
  > - `payg address` — prints the wallet address (no network call, fast). Agents need this for display/logging.
  > - `payg balance` — checks USDC + ETH balance on Base (requires RPC call). Agents need this to verify they can pay.
  > - `payg charge --dry-run` — validates config, wallet, ceiling without sending payment. Agents use this for pre-flight checks.
- [ ] Implement `payg init` `[crates/payg-cli/src/commands/init.rs]`
  - Generate new encrypted keyfile at `~/.payg/keyfile.enc`
  - Prompt for password (or `PAYG_KEY_PASSWORD` env var for non-interactive)
  - Write `~/.payg/config.toml` with keyfile path and defaults
  - Print wallet address on success
  - `--non-interactive`: use env vars, no prompts
- [ ] Implement `payg charge` `[crates/payg-cli/src/commands/charge.rs]`
  - Direct mode: `payg charge 0.001 --recipient 0xABC --token USDC`
  - Config mode: reads payg.toml, charges based on `--command <name>`
  - JSON output on stdout: `{"status":"success","tx_hash":"0x...","amount":"0.001 USDC"}`
  - Text output on stderr for human context: `Charged 0.001 USDC -> 0xABC (tx: 0x123...)`
  - Exit code 0 = success, non-zero = failure (see exit code table below)
  > **Research Insight — Agent-Native Architecture**: Use exit code `78` (BSD `EX_CONFIG`) for config errors, `77` (`EX_NOPERM`) for wallet issues, and a custom `42` for payment-specific failures. Structured JSON errors should include a machine-readable `code` field: `{"status":"error","code":"EXCEEDS_CEILING","message":"..."}`. This lets agents branch on error types without parsing strings.
- [ ] Implement `payg address` `[crates/payg-cli/src/commands/address.rs]`
  - Load wallet, print address to stdout
  - JSON mode: `{"address":"0xABC..."}`
  - No network call — fast, useful for agents
- [ ] Implement `payg balance` `[crates/payg-cli/src/commands/balance.rs]`
  - Load wallet, query Base RPC for ETH + USDC balances
  - JSON mode: `{"address":"0xABC...","eth":"0.05","usdc":"12.34","chain":"base"}`
  - Text mode: `Wallet 0xABC...DEF: 0.05 ETH / 12.34 USDC (Base)`
- [ ] Implement `--dry-run` flag for `payg charge` `[crates/payg-cli/src/commands/charge.rs]`
  - Validates config, wallet, ceiling, recipient — does NOT send payment
  - Returns what would happen: `{"status":"dry_run","amount":"0.001 USDC","recipient":"0x..."}`
- [ ] Write CLI integration tests with `assert_cmd` `[crates/payg-cli/tests/]`

**Success criteria:** `payg init` creates a wallet. `payg charge` sends payment and returns result. `payg address` and `payg balance` work. JSON and text output modes work.

#### Phase 3: Examples, Distribution & Polish

End-to-end examples, Homebrew distribution, and documentation.

**Tasks — Examples:**

- [ ] Create `examples/simple-cli/` — a Rust CLI tool using `payg::charge()` `[examples/simple-cli/]`
  - 3-4 commands with different prices
  - Shows the one-line `payg::charge()` integration
  - Includes payg.toml with per-command pricing
- [ ] Create `examples/python-tool/` — a Python script using payg CLI binary `[examples/python-tool/]`
  - Demonstrates `payg.toml` config
  - Shows shell-out to `payg charge` subprocess
  - Includes setup instructions

**Tasks — Distribution:**

- [ ] Run `dist init` to configure cargo-dist `[Cargo.toml workspace.metadata.dist]`
  - Targets: x86_64-linux, x86_64-darwin, aarch64-darwin
  - Installers: shell, homebrew
  - Tap: `<org>/homebrew-tap`
- [ ] Create Homebrew tap repository on GitHub
- [ ] Set up release automation: release-plz + git-cliff `[release.toml, cliff.toml]`
- [ ] Create GitHub Actions release workflow `[.github/workflows/release.yml]`
- [ ] Verify `brew install <org>/tap/payg` works end-to-end

**Tasks — Documentation:**

- [ ] Write README.md with quickstart for both Rust and non-Rust developers `[README.md]`
- [ ] Write CLAUDE.md with project conventions `[CLAUDE.md]`
- [ ] Add rustdoc comments to all public APIs

**Tasks — Testing:**

- [ ] End-to-end test: Rust CLI tool charges per invocation on Base Sepolia, payment lands in wallet
- [ ] End-to-end test: Non-Rust tool with payg.toml triggers payment via `payg charge` subprocess
- [ ] End-to-end test: `payg init` → fund wallet → use a PAYG-enabled tool

**Success criteria:** All three v1 success criteria met:
1. End-to-end demo works on Base Sepolia
2. `cargo add payg` works, developer has billing in <10 minutes
3. `brew install payg` works, non-Rust devs use payg.toml

## V1.1 Roadmap (Post-Launch)

Items deferred from v1 based on review feedback. Add when real users validate the need:

| Feature | Trigger to Build | Crate Impact |
|---|---|---|
| `#[payg(cost = "0.001")]` proc macros | Users confirm charge-then-execute pattern works | Add `payg-macros` crate |
| On-demand daemon (Unix socket) | Subprocess latency is a real complaint (esp. keyfile path) | Add `daemon/` module to payg-cli |
| WalletProvider trait + multi-provider | Users ask for CDP v2, 1Password, Foundry | Refactor `wallet.rs` to trait |
| Per-command `[commands]` pricing in payg.toml | Developers need per-command pricing | Add config parsing + `--command` flag |
| `payg wallets discover` | Agent runtimes need wallet enumeration | Add to payg-cli |
| `payg fund` / `payg price` / `payg context` | Users request them | Add to payg-cli |
| `payg` facade crate (re-export) | If `cargo add payg` needs simpler imports | Add thin facade crate |
| Concurrent nonce management | Parallel agents sharing one wallet | Add nonce mutex to ETH backend |

> **Research Insight — Agent-Native Architecture**: `payg context` (v1.1) would output a system prompt block that agents can inject: tool name, pricing, accepted tokens, wallet address, balance. This enables agents to reason about costs before committing. Also consider `payg.toml` -> MCP tool manifest generation for AI tool registries.

## Alternative Approaches Considered

- **6 crates (original plan)** — rejected after simplicity review: premature abstraction for v1. Two feature flags are not complexity.
- **x402-reqwest as payment API** — rejected after architecture review: x402-reqwest is HTTP middleware for intercepting 402 responses, not a direct payment API. Must use x402-chain-eip155 directly.
- **Daemon from v1** — rejected: charge latency is dominated by 500ms-2000ms network I/O. Subprocess spawn overhead (~50-100ms) is invisible in that context.
- **Proc macros from v1** — rejected: saves one line per function, adds heavy compile-time deps and complexity. Ship the primitive first.
- **Custom payment protocol** — rejected: x402 is an open standard with ecosystem momentum (Coinbase, Cloudflare, Google)
- **Custom wallet pattern** — rejected: ecosystem has converged on TEE-managed server wallets. Build the trait when we have 3+ providers asking for it.

## Acceptance Criteria

### Functional Requirements

- [ ] `payg::charge("0.001 USDC", recipient)` executes a payment on Base L2
- [ ] x402 backend: signs EIP-3009 authorization via `x402-chain-eip155`, POSTs to facilitator `/settle`
- [ ] Direct ETH backend: sends native ETH transfer via alloy-rs
- [ ] `payg.toml` defines per-command pricing, read by CLI binary
- [ ] `~/.payg/config.toml` configures wallet keyfile and safety ceiling
- [ ] `payg init` generates encrypted keyfile and writes config
- [ ] `payg init --non-interactive` uses env vars, no prompts
- [ ] `payg charge` sends payment via subprocess and returns result
- [ ] Safety ceiling: charges above $1.00 USDC rejected by default
- [ ] JSON output mode (`--output json`) for CLI commands

### Non-Functional Requirements

- [ ] MSRV: 1.88.0 (matches x402-rs)
- [ ] All public APIs documented with rustdoc
- [ ] `cargo clippy --workspace -- -D warnings` passes
- [ ] `cargo fmt --all -- --check` passes
- [ ] CI runs on every PR: test, clippy, fmt, MSRV check

### Quality Gates

- [ ] Unit tests for: config parsing, price parsing, safety ceiling
- [ ] Integration tests for: x402 payment on Sepolia, ETH payment on Sepolia
- [ ] CLI tests with `assert_cmd`: init, charge, JSON output, error handling
- [ ] End-to-end test: full payment flow from `payg::charge()` to on-chain settlement

## Dependencies & Prerequisites

| Dependency | Status | Risk |
|---|---|---|
| x402-rs crate (v1.1.0) | Available on crates.io | Low — Apache-2.0, active development |
| x402-chain-eip155 | Available on crates.io | Low — part of x402-rs workspace |
| alloy-rs (v1.4) | Available on crates.io | Low — maintained by Paradigm |
| Base Sepolia testnet | Public | Low — free testnet |
| x402 testnet facilitator | Public | Medium — community-run |
| Homebrew tap | Needs creation | Low — straightforward |
| cargo-dist | Available | Low — actively maintained by axo.dev |

## Risk Analysis & Mitigation

| Risk | Likelihood | Impact | Mitigation |
|---|---|---|---|
| x402-rs API changes | Low | Medium | Pin version, watch releases |
| x402-rs Issue #26 (gas estimation on OP-stack) | Low | High | Facilitator-side fix; use `estimateGas.block(latest)` |
| Base L2 gas spikes | Medium | Low | x402 shifts gas to facilitator; monitor |
| Testnet facilitator downtime | Medium | Medium | `facilitator_url` configurable; mock for tests |
| Payment sent but receipt timeout | Medium | High | Log tx_hash before awaiting; include in error for manual verification |
| Cross-platform (Windows) | High | Low | Unix-only for v1; Windows named pipes in future |
| MSRV 1.88.0 too recent | Low | Medium | Edition 2024 + x402-rs require it; document clearly |
| Binary size (15-30MB unoptimized) | Medium | Low | Release profile: strip, lto, opt-level=s; individual alloy sub-crates |

> **Research Insight — Security Review**: Key security controls for v1: (1) never log private keys or passwords, even at debug level; (2) validate config file permissions — `~/.payg/keyfile.enc` should be 0600; (3) the safety ceiling check MUST happen before any signing or network call (fail-fast); (4) validate recipient addresses before payment (checksum validation); (5) use `secrecy` crate or `zeroize` for private key memory if concerned about core dumps.

## Future Considerations

Documented in brainstorm, not in v1 scope:
- Full guardrails (spending limits, confirmations, audit logs, budget caps)
- Relay/batching service for sub-cent aggregation
- Additional chains (Lightning, Solana, mainnet ETH)
- FFI bindings (C ABI for Python/Go/JS)
- ERC-7710 delegation (human wallet delegates to agent)
- ERC-8004 agent identity (on-chain reputation)
- Refund/dispute model
- Concurrent transaction management (nonce mutex for parallel agents)
- HTTP middleware (use x402-axum directly for now)

## References & Research

### Internal References

- Brainstorm: `docs/brainstorms/2026-02-09-payg-billing-system-brainstorm.md`
- Gas research: `docs/research/base-l2-gas-costs-research.md`

### External References — Local Repos

- x402-rs source: `~/github-stars/x402-rs/x402-rs/` — Facilitator trait, EIP-3009 signing, X402Client
- alloy-rs source: `~/github-stars/alloy-rs/alloy/` — ProviderBuilder, TransactionRequest, PrivateKeySigner

### External References — Documentation

- x402 protocol: https://x402.org / https://github.com/coinbase/x402
- x402-rs crate: https://github.com/x402-rs/x402-rs
- x402 facilitator: https://facilitator.x402.rs
- alloy-rs: https://alloy.rs / https://github.com/alloy-rs/alloy
- cargo-dist: https://github.com/axodotdev/cargo-dist
- Foundry key management: https://getfoundry.sh/guides/best-practices/key-management/
- eth-keystore crate: https://docs.rs/eth-keystore

### Related Standards

- EIP-3009 (transferWithAuthorization): https://eips.ethereum.org/EIPS/eip-3009
- EIP-712 (typed data signing): https://eips.ethereum.org/EIPS/eip-712
- CAIP-2 (chain identifiers): https://github.com/ChainAgnostic/CAIPs/blob/main/CAIPs/caip-2.md

### AI-Era Notes

- Brainstorming conducted with Claude Opus 4.6 via Claude Code
- x402 deep dive, gas economics validation, agent wallet landscape research all AI-assisted
- Architecture review caught critical x402-reqwest misunderstanding before implementation
- Simplicity review reduced plan from 6 crates to 2 crates (~85% LOC reduction)
- `/deepen-plan` run (2026-02-10) launched 8 parallel research/review agents:
  - x402 EIP-3009 signing API research — concrete `sign_erc3009_authorization()` usage, `SignerLike` trait, `Eip3009SigningParams` struct
  - alloy-rs ETH transfer patterns — `ProviderBuilder`, `TransactionRequest`, `Eip1559Estimation`, `CachedNonceManager`
  - Security review — keyfile permissions, config validation, private key handling, fail-fast ceiling checks
  - Performance review — scrypt KDF 200-500ms overhead, subprocess latency correction, binary size optimization, current_thread tokio
  - Architecture review — module layout, feature-flag leak, `compile_error!` guard, `facilitator_url`, Token decimal metadata
  - Simplicity review — Backend enum over trait (-20 LOC, -1 dep), simplified ChargeRequest/Receipt, deferred per-command pricing
  - Pattern recognition — naming conventions correct, `eth-keystore` version pin, `serde_json` missing dependency, async_trait unnecessary
  - Agent-native architecture — `payg address`/`balance`/`--dry-run` commands, structured JSON errors, exit code conventions, `payg context` (v1.1)
