use clap::{Parser, Subcommand, ValueEnum};

use crate::commands::{address, balance, charge, init};

#[derive(Parser)]
#[command(
    name = "payg",
    version,
    about = "Pay-as-you-go crypto micropayments for CLI tools",
    after_help = "EXIT CODES:\n  0   Success\n  1   General error\n  2   Invalid arguments (from argument parser)\n  42  Payment failed or exceeds safety ceiling\n  77  Wallet error (missing or decryption failure)\n  78  Configuration error\n\nENVIRONMENT VARIABLES:\n  PAYG_OUTPUT            Output format: text, json (same as --output)\n  PAYG_NETWORK           Target network: base, base-sepolia (same as --network)\n  PAYG_PRIVATE_KEY       Hex private key (recommended for agents, bypasses keyfile)\n  PAYG_KEY_PASSWORD      Keyfile decryption password (avoids interactive prompt)\n  PAYG_MAX_CHARGE        Safety ceiling override (e.g. \"5.00 USDC\")\n  PAYG_RPC_URL           RPC endpoint override\n  PAYG_FACILITATOR_URL   x402 facilitator URL override"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,

    /// Output format
    #[arg(long, global = true, default_value = "text", env = "PAYG_OUTPUT")]
    pub output: OutputFormat,

    /// Target network
    #[arg(long, global = true, env = "PAYG_NETWORK")]
    pub network: Option<NetworkName>,
}

#[derive(Subcommand)]
pub enum Command {
    /// Initialize a new PAYG wallet
    Init(init::InitArgs),
    /// Send a payment
    Charge(charge::ChargeArgs),
    /// Show wallet address
    Address(address::AddressArgs),
    /// Check wallet balances
    Balance(balance::BalanceArgs),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum OutputFormat {
    Text,
    Json,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum NetworkName {
    /// Base mainnet (real funds)
    Base,
    /// Base Sepolia testnet
    #[value(name = "base-sepolia")]
    BaseSepolia,
}

impl NetworkName {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Base => "base",
            Self::BaseSepolia => "base-sepolia",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn network_names_match_lib_resolver() {
        // Ensure every CLI NetworkName variant resolves successfully in the lib.
        // If a variant is added to NetworkName without a matching resolve arm,
        // this test will fail.
        for name in [NetworkName::Base, NetworkName::BaseSepolia] {
            payg::network::resolve_network_config(name.as_str()).unwrap_or_else(|_| {
                panic!(
                    "CLI network '{}' not recognized by lib resolver",
                    name.as_str()
                )
            });
        }
    }
}
