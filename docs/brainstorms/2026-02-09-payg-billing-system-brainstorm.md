---
date: 2026-02-09
topic: payg-billing-system
---

# PAYG: Pay-As-You-Go Crypto Micropayments for CLI Tools

## What We're Building

A drop-in pay-as-you-go billing system that developers can integrate into their CLI tools and libraries. Built in Rust, distributed as both a crate (for Rust developers) and a standalone binary (for any language). Payments are peer-to-peer crypto micropayments targeting the coming wave of agentic workflows, where autonomous agents are more likely to pay fractions of a cent per tool invocation than human users ever would.

The core insight: PAYG is a **payment primitive**, not an opinionated billing framework. The developer (or agent) controls when charges happen. PAYG handles how to pay.

**PAYG's value proposition**: x402 solves the HTTP API payment case. PAYG solves the CLI tool case. They are complementary layers. A developer shouldn't need to understand HTTP 402, EIP-3009, or facilitators. They add `#[payg(cost = "0.001")]` or drop in a `payg.toml` and they're done.

## Why This Approach

### Approaches Considered

1. **Rust Workspace Monorepo** (chosen) — Cargo workspace with separate crates per concern. Idiomatic, independently publishable, clean separation.
2. **Single Crate + Feature Flags** — Simpler to start but feature flag complexity grows fast. Cross-compilation gets messy.
3. **Separate Repos** — Cleanest separation but versioning coordination overhead.

### Why Workspace Monorepo

- Idiomatic Rust project structure for multi-artifact projects
- Each crate is independently versioned and publishable to crates.io
- Unified build and test via `cargo workspace`
- The CLI binary naturally embeds the core crate
- Maps directly to the layered architecture

## Key Decisions

- **Core primitive**: `payg::charge(amount, recipient, metadata)` — everything else is sugar on top
- **Three-tier architecture**: Primitive (raw `charge()`), Patterns (CLI wrapping, HTTP 402, proc macros), Policies (guardrails, budgets, limits). The primitive is the foundation; patterns and policies are built on top.
- **Payment backends**: x402 (USDC via EIP-3009, zero client gas) + direct on-chain ETH transfer (via alloy-rs). Two backends from v1. Chain-agnostic trait allows adding more later.
- **x402 integration**: PAYG uses x402 as its primary payment backend. x402 is an open standard (Apache-2.0), self-hostable, with ecosystem backing (Coinbase, Cloudflare, Google A2A). x402-rs provides the Rust implementation. PAYG adds the CLI/developer DX layer that x402 doesn't address.
- **Chain target**: Base L2. Gas costs $0.0001-$0.003/tx. Sub-cent micropayments are viable. Lightning as future backend.
- **Distribution**: Embedded binary is the happy path (ships inside the host tool). System install (`brew install payg`) for developers building integrations.
- **Wallet strategy**: Wallet-provider agnostic. PAYG does NOT create its own wallet pattern. Supports CDP v2 (TEE), Turnkey, 1Password `op://`, Foundry keystore import, local encrypted keyfile. Auto-detects existing wallets on the system.
- **Developer DX**: Rust devs get proc macros (`#[payg(cost = "0.001")]`). All other languages get payg.toml + embedded binary.
- **Binary invocation**: On-demand daemon. Auto-spawns on first `payg charge` call via Unix socket at `~/.payg/payg.sock`. Auto-shuts down after 60s of inactivity. No systemd, no persistent service, no user approval needed.
- **Trigger model**: Event-based at the core. Built-in patterns for CLI invocation, HTTP 402 interception, and custom events. Future patterns: per-token streaming, time-based, per-byte.
- **Charge-execution ordering**: Charge-then-execute for v1 (consumer bears risk of tool failure). Simplest model, avoids "free work" problem.
- **Minimum safety for v1**: Default max-charge-per-invocation ceiling ($1.00 USDC). Configurable by consumer. Safety net, not full guardrail system.

## Architecture

```
Developer DX Layer (PAYG)
  ├── Rust proc macros (#[payg(cost = "0.001")])
  ├── payg.toml + CLI binary (cross-language)
  ├── Wallet management (init, discover, auto-detect)
  ├── On-demand daemon (Unix socket, auto-spawn/shutdown)
  └── Price discovery, onboarding, safety ceiling
      │
Payment Backend Layer (chain-agnostic trait)
  ├── x402 backend (Base/USDC — default, zero client gas)
  ├── Direct ETH backend (Base/ETH via alloy-rs — any EVM chain)
  └── Lightning backend (sats — future)
```

### Crate Structure

```
payg/
  crates/
    payg-core/       # Payment primitive, chain trait, wallet provider trait
    payg-macros/     # Proc macros: #[payg(cost = "...")]
    payg-cli/        # Standalone binary, daemon, reads payg.toml
    payg-http/       # HTTP 402 middleware/interceptor
    payg-x402/       # x402 backend (via x402-rs)
    payg-eth/        # Direct ETH transfer backend (via alloy-rs)
  docs/
  examples/
  Cargo.toml         # Workspace root
```

### Three-Tier Model

| Tier | Responsibility | Examples |
|---|---|---|
| **Primitive** | Raw payment operation | `payg::charge()`, chain trait, wallet provider trait |
| **Patterns** | Standard integrations built on the primitive | CLI wrapping, HTTP 402, proc macros, payg.toml |
| **Policies** | Controls on pattern behavior | Spending limits, budgets, confirmations, audit logs |

### Payment Trigger Patterns

| Pattern | Description |
|---|---|
| CLI invocation | Charge per command execution |
| HTTP 402 | Auto-pay on 402 responses via x402 |
| Per-token | Charge during LLM streaming (future) |
| Time-based | Charge on intervals during sessions (future) |
| Custom | Developer calls `payg::charge()` anywhere |

### Chain Abstraction Note

EVM chains (Base) use push-payment (payer initiates transfer). Lightning uses pull-payment (recipient generates invoice, payer pays). x402 uses a hybrid: client signs off-chain authorization (push-intent), facilitator settles on-chain. The chain trait must accommodate these models. x402's EIP-3009 approach (client signs, facilitator settles) is the cleanest for micropayments because the client pays zero gas.

## Config Schemas

### Developer Config: `payg.toml`

```toml
[payg]
version = "1"
recipient = "0x1234...abcd"         # developer's wallet address
chain = "base"                       # default chain
description = "My awesome CLI tool"

[payg.pricing]
default = "0.001 USDC"              # default cost per invocation

[payg.pricing.commands]
generate = "0.01 USDC"              # per-command overrides
"generate --large" = "0.05 USDC"
validate = "free"                    # explicitly free

[payg.accept]
usdc = true                          # via x402
eth = true                           # via direct transfer
```

### Consumer Config: `~/.payg/config.toml`

```toml
[wallet]
# Wallet provider (auto-detected or manually configured)
# Options: "cdp-v2", "turnkey", "local", "op", "foundry"
provider = "cdp-v2"

# Provider-specific config
[wallet.cdp-v2]
api_key_id = "..."
api_secret = "..."

# OR local encrypted keyfile
# [wallet.local]
# keyfile = "~/.payg/keyfile.enc"

# OR 1Password reference
# [wallet.op]
# reference = "op://personal/payg-wallet/private-key"

# OR import from Foundry
# [wallet.foundry]
# keystore = "~/.foundry/keystores/deployer"

[payment]
protocol = "x402"
facilitator = "https://x402.org/facilitator"  # or self-hosted

[safety]
max_charge = "1.00 USDC"             # reject single charges above this
# max_daily = "10.00 USDC"           # future: daily budget cap
```

## Consumer Onboarding Flows

### `payg init` (Interactive — Human)

```
$ payg init
Scanning for existing wallets...
  Found: Foundry keystore "deployer" (~/.foundry/keystores/deployer)
  Found: 1Password CLI available
  Found: Coinbase CDP credentials configured

Which wallet would you like to use?
  1. Coinbase CDP v2 (recommended — TEE-backed, most secure)
  2. Import from Foundry keystore "deployer"
  3. Reference via 1Password
  4. Create new local encrypted keyfile
> 1
Configured! Wallet 0xABC...DEF on Base.
```

### `payg init --non-interactive` (Agent)

Auto-detects wallets, picks most secure option (Tier 1 TEE > Tier 3 encrypted > Tier 4 raw).

### `payg wallets discover` (Agent-Native Discovery)

```json
{"wallets":[
  {"provider":"cdp-v2","tier":1,"address":"0xABC..."},
  {"provider":"foundry","tier":3,"name":"deployer","path":"~/.foundry/keystores/deployer"},
  {"provider":"op","tier":3,"available":true}
]}
```

Machine-readable JSON for agents to parse and make decisions.

### First-Run (No Wallet Configured)

PAYG detects missing wallet, prints clear setup instructions, does NOT execute the paid tool. Developer's tool receives `PaymentError::NoWallet` to handle gracefully.

### Price Discovery

`payg price <command>` returns cost. `payg.toml` is machine-readable. Agents query costs before committing.

## Wallet Security Tiers

| Tier | Approach | Examples | When to Use |
|---|---|---|---|
| 1 (best) | TEE server wallets | CDP v2, Turnkey, Privy | Production agents |
| 2 | MPC/distributed | Lit Protocol, Circle | Decentralized agents |
| 3 | Secret manager / encrypted file | 1Password `op://`, local keyfile, Foundry keystore | Development, self-hosted |
| 4 (warn) | Raw private key | Env var, plaintext file | Testing only — PAYG warns |

Key Rust crates: `keyring` (OS keychain), `eth-keystore` (Web3 V3 format), `secrecy`/`zeroize` (memory safety), `alloy-signer-local` (keystore signing).

## V1 Success Criteria

1. **End-to-end demo**: A sample CLI tool charges per invocation, payment lands in a wallet on Base L2 via x402
2. **Published crate**: `cargo add payg` works, developer has billing in their tool in <10 minutes
3. **Brew-installable**: `brew install payg` gives a working binary, non-Rust devs add payg.toml and start charging

## Gas Economics (Validated)

| Transaction Type | Base L2 Cost |
|---|---|
| Simple ETH transfer | $0.0001 - $0.001 |
| ERC-20 (USDC) transfer | $0.0005 - $0.003 |
| x402 settlement (CDP facilitator) | Free (1,000/mo) then $0.001/tx |
| x402 settlement (self-hosted) | Gas only (~$0.001) |

Sub-cent micropayments are viable on Base L2. Caveat: gas can spike 10-100x during congestion. x402 shifts gas cost from client to facilitator.

## x402 Protocol Summary

- **What**: Open HTTP-native payment standard (Apache-2.0). Client signs EIP-3009 authorization off-chain, facilitator settles on-chain.
- **Ecosystem**: Coinbase, Cloudflare (native Workers support), Google (A2A/AP2), Circle, Dynamic. 75M+ transactions, $24M+ processed.
- **Limitation**: USDC-only on EVM (EIP-3009 dependency). Broader token support targeted Q2 2026.
- **Rust crate**: x402-rs (community, production-grade). Includes facilitator, axum middleware, reqwest client.
- **Lock-in risk**: Low. Open spec, self-hostable facilitators, multiple implementations. Real dependency is on USDC's EIP-3009 support (controlled by Circle).
- **Why PAYG still adds value**: x402 is HTTP-native. PAYG makes it work for CLI tools — proc macros, payg.toml, wallet management, on-demand daemon, price discovery, agent-first onboarding.

## Future Considerations (Not V1)

- **Full guardrails**: Spending limits, confirmation prompts, audit logs, budget caps
- **Relay/batching service**: Aggregate micropayments to reduce gas costs
- **Additional chains**: Lightning Network, mainnet ETH, other L2s, Solana
- **FFI bindings**: C ABI for deeper integration from Python, Go, JS
- **Agent spending policies**: Budget management, rate limiting, approval thresholds
- **ERC-7710 delegation**: Human wallet delegates scoped spending authority to agent
- **ERC-8004 agent identity**: On-chain reputation for agent wallets
- **Refund/dispute model**: What happens when a tool fails after payment?
- **Receipt/proof-of-payment**: Transaction hashes, local receipt logs
- **Concurrent transaction management**: Nonce management for parallel agent usage
- **Protocol versioning**: Compatibility between PAYG versions
- **System install mode**: Shared PAYG installation for developer workflows

## Open Questions

- **Idempotency**: How to prevent duplicate payments when agents retry? x402 nonces help but application-level dedup may also be needed.
- **Pricing denomination**: Human-readable strings like "0.001 USDC" in config. Native token amounts for ETH backend. Conversion handled at charge time.
- **Testnet story**: Base Sepolia for testing. x402.org provides a testnet facilitator. `PAYG_NETWORK=testnet` or `chain = "base-sepolia"` in config.
- **Offline behavior**: Fail fast with clear error. No offline queuing in v1.
- **Dynamic pricing**: Per-command static pricing for v1. Dynamic pricing (cost_fn) as future enhancement.
- **Regulatory surface**: P2P crypto transfers with key storage have compliance implications. Acknowledged as constraint.
- **Embedded binary distribution**: Cross-platform binary distribution for non-Rust ecosystems (PyPI, npm) is non-trivial. Needs explicit design during planning.

## Next Steps

-> `/workflows:plan` for implementation details

## Research Artifacts

Detailed research documents generated during this brainstorm:
- `docs/base-l2-gas-costs-research.md` — Base L2 gas economics analysis
