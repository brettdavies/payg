use clap::{Parser, Subcommand, ValueEnum};

use crate::commands::{address, balance, charge, init};

#[derive(Parser)]
#[command(
    name = "payg",
    about = "Pay-as-you-go crypto micropayments for CLI tools"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,

    /// Output format
    #[arg(long, global = true, default_value = "text")]
    pub output: OutputFormat,
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
