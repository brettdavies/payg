//! Simple CLI tool demonstrating PAYG integration.
//!
//! Two subcommands:
//! - `greet <name>` — charges 0.001 USDC, prints a greeting
//! - `count-words <text>` — free command, no charge

use std::env;
use std::process;

/// WARNING: Hardhat account #0 — a well-known test address whose private key is public.
/// Any real funds sent here can be stolen by anyone.
/// REPLACE with your own address before using on mainnet.
const RECIPIENT: &str = "0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266";

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        eprintln!("Usage: simple-cli <greet|count-words> [args...]");
        process::exit(1);
    }

    match args[1].as_str() {
        "greet" => {
            let name = args.get(2).map(|s| s.as_str()).unwrap_or("world");
            match payg::charge("0.001 USDC", RECIPIENT).await {
                Ok(receipt) => {
                    println!("Hello, {name}! (tx: {})", receipt.tx_hash);
                }
                Err(e) => {
                    eprintln!("Payment failed: {e}");
                    process::exit(42);
                }
            }
        }
        "count-words" => {
            // Free command — no charge
            let text = args[2..].join(" ");
            let count = text.split_whitespace().count();
            println!("{count} words");
        }
        other => {
            eprintln!("Unknown command: {other}");
            eprintln!("Usage: simple-cli <greet|count-words> [args...]");
            process::exit(1);
        }
    }
}
