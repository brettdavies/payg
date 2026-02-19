# PAYG

Pay-as-you-go crypto micropayments for CLI tools. Charge per invocation on Base L2 with USDC (via [x402](https://www.x402.org/)) or direct ETH transfers.

## How It Works

**Tool developers** add a `payg.toml` to their project and call `payg::charge()` (Rust) or shell out to `payg charge` (any language).

**End users** run `payg init` once to create a wallet, fund it with USDC on Base, and every tool invocation automatically pays the developer.

Payments go peer-to-peer on Base L2. Gas costs $0.001 or less per transaction.

## Install

### From source

```sh
cargo install payg-cli
```

### As a library

```sh
cargo add payg
```

Requires Rust 1.88.0+.

## Quick Start

### For end users

```sh
# Create a wallet (defaults to Base Sepolia testnet)
payg init

# Or target mainnet directly
payg init --network base

# Fund the address with USDC on Base (shown after init)
# For testnet USDC: https://faucet.circle.com/
# Then use any PAYG-enabled tool — payments happen automatically
```

### For tool developers (Rust)

Add billing to your CLI in 3 lines:

```rust
// In your tool's main function
payg::charge("0.001 USDC", "0xYOUR_WALLET_ADDRESS").await?;
```

Create a `payg.toml` in your project root:

```toml
recipient = "0xYOUR_WALLET_ADDRESS"
default_price = "0.001 USDC"
```

### For tool developers (any language)

Shell out to the `payg` binary:

```sh
payg charge "0.001 USDC" --recipient 0xYOUR_WALLET_ADDRESS
```

Or rely on `payg.toml` defaults:

```sh
payg charge
```

## CLI Commands

### Global flags

All commands accept these flags:

| Flag | Env var | Default | Description |
|------|---------|---------|-------------|
| `--output` | `PAYG_OUTPUT` | `text` | Output format: `text`, `json` |
| `--network` | `PAYG_NETWORK` | `base-sepolia` | Target network: `base`, `base-sepolia` |

Precedence: CLI flag > env var > config file > default.

### `payg init`

Create a new wallet at `~/.payg/keyfile.json`.

```sh
payg init                              # Interactive (prompts for password)
payg init --network base               # Create wallet targeting mainnet
payg init --non-interactive            # Requires PAYG_KEY_PASSWORD env var
payg init --non-interactive --output json
```

### `payg charge`

Send a payment.

```sh
payg charge "0.001 USDC" --recipient 0x1234...
payg charge                            # Uses payg.toml defaults
payg charge --network base             # Charge on mainnet
payg charge --dry-run                  # Validate without sending
payg charge --dry-run --output json    # Machine-readable validation
```

### `payg address`

Show wallet address.

```sh
payg address
payg address --output json             # {"address": "0x..."}
```

### `payg balance`

Check ETH and USDC balances.

```sh
payg balance
payg balance --network base            # Check mainnet balances
payg balance --output json
```

## Configuration

### Project config (`payg.toml`)

Created by tool developers in the project root:

```toml
recipient = "0xYOUR_WALLET_ADDRESS"
default_price = "0.001 USDC"
```

| Field | Required | Description |
|-------|----------|-------------|
| `recipient` | Yes | Wallet address to receive payments |
| `default_price` | No | Default charge amount (e.g. `"0.001 USDC"`) |

### User config (`~/.payg/config.toml`)

Created automatically by `payg init`:

```toml
keyfile = "~/.payg/keyfile.json"
max_charge = "1.00 USDC"
network = "base-sepolia"
```

| Field | Default | Description |
|-------|---------|-------------|
| `keyfile` | `~/.payg/keyfile.json` | Path to encrypted keyfile |
| `max_charge` | `1.00 USDC` | Safety ceiling per charge |
| `network` | `base-sepolia` | Default network (`base`, `base-sepolia`) |
| `facilitator_url` | `https://x402.org/facilitator` | x402 facilitator endpoint |
| `rpc_url` | Per-network default | Base RPC endpoint override |

### Environment variables

All config values can be overridden with environment variables:

| Variable | Description |
|----------|-------------|
| `PAYG_PRIVATE_KEY` | Hex private key (skips keyfile, recommended for agents) |
| `PAYG_KEY_PASSWORD` | Keyfile password (skips interactive prompt) |
| `PAYG_NETWORK` | Target network: `base`, `base-sepolia` (same as `--network`) |
| `PAYG_OUTPUT` | Output format: `text`, `json` (same as `--output`) |
| `PAYG_MAX_CHARGE` | Safety ceiling override (e.g. `"5.00 USDC"`) |
| `PAYG_RPC_URL` | Base RPC endpoint override |
| `PAYG_FACILITATOR_URL` | x402 facilitator URL override |

Precedence: CLI flag > env var > config file > default.

**For CI/agents**, set `PAYG_PRIVATE_KEY` to avoid keyfile decryption overhead (200-500ms scrypt vs 10-30ms env var).

## Library API

```rust
use payg::{charge, charge_with_config, ConsumerConfig, NetworkConfig, PaygError};

// Simple: loads config and wallet automatically
payg::charge("0.001 USDC", "0xRECIPIENT").await?;

// Pre-loaded: avoids redundant config/wallet loading in loops
let config = ConsumerConfig::load()?;
let network = config.resolve_network()?;
let signer = payg::wallet::load_wallet(&config, Some("password"))?;
let receipt = payg::charge_with_config(
    "0.001 USDC",
    "0xRECIPIENT",
    &config,
    network,
    signer,
).await?;
println!("tx: {}", receipt.tx_hash);
```

### Price formats

| Format | Example | Result |
|--------|---------|--------|
| USDC | `"0.001 USDC"` | 1000 units (6 decimals) |
| ETH | `"0.01 ETH"` | 10^16 wei (18 decimals) |
| Free | `"free"` | 0 |

### Feature flags

```toml
[dependencies]
payg = "0.1"                          # Default: x402 (USDC) backend
payg = { version = "0.1", features = ["eth"] }  # Add ETH backend
```

| Feature | Default | Backend |
|---------|---------|---------|
| `x402` | Yes | USDC via ERC-3009 + x402 facilitator |
| `eth` | No | Direct ETH transfer via alloy |

## Exit Codes

| Code | Meaning |
|------|---------|
| 0 | Success |
| 1 | General error |
| 2 | Invalid arguments |
| 42 | Payment failed or exceeds safety ceiling |
| 77 | Wallet error (missing or decryption failure) |
| 78 | Configuration error |

## Agent Integration

PAYG is designed for agent-native usage:

- `--output json` (or `PAYG_OUTPUT=json`) on every command for structured output
- All JSON responses include `status`, `network`, and `is_testnet` fields
- All errors include a `code` field (`NO_WALLET`, `EXCEEDS_CEILING`, `INVALID_ARGS`, etc.)
- `PAYG_PRIVATE_KEY` env var for fast, non-interactive wallet access
- `PAYG_NETWORK` env var to select network without CLI flags
- `--dry-run` to validate charges before sending
- Semantic exit codes for programmatic error handling

```sh
# Agent workflow
export PAYG_PRIVATE_KEY=0x...
export PAYG_OUTPUT=json
export PAYG_NETWORK=base-sepolia

payg balance
payg charge "0.001 USDC" --recipient 0x... --dry-run
payg charge "0.001 USDC" --recipient 0x...
```

## Security

- Keyfiles encrypted with scrypt, stored with `0600` permissions
- `~/.payg/` directory created with `0700` permissions
- Private keys zeroized from memory after use
- URLs validated to require HTTPS (localhost exception for development)
- Safety ceiling checked before wallet load (fail-fast)
- ERC-3009 authorization timeout capped at 120 seconds

## License

Dual-licensed under [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE), at your option.
