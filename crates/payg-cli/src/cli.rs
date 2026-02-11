use clap::{Parser, Subcommand, ValueEnum};

use crate::commands::{address, balance, charge, init};

#[derive(Parser)]
#[command(
    name = "payg",
    version,
    about = "Pay-as-you-go crypto micropayments for CLI tools",
    after_help = "EXIT CODES:\n  0   Success\n  1   General error\n  2   Invalid arguments (from argument parser)\n  42  Payment failed or exceeds safety ceiling\n  77  Wallet error (missing or decryption failure)\n  78  Configuration error"
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
