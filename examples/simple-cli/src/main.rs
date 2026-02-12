//! Simple CLI tool demonstrating PAYG integration.
//!
//! Three subcommands:
//! - `greet <name>` — charges default_price from payg.toml, prints a greeting
//! - `fortune` — charges an explicit "0.002 USDC", prints a fortune
//! - `count-words <text>` — free command, no charge

use std::env;
use std::process;

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        eprintln!("Usage: simple-cli <greet|fortune|count-words> [args...]");
        process::exit(1);
    }

    match args[1].as_str() {
        "greet" => {
            let name = args.get(2).map(|s| s.as_str()).unwrap_or("world");
            // Uses default_price from payg.toml (0.001 USDC)
            let recipient = "0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266";
            match payg::charge("0.001 USDC", recipient).await {
                Ok(receipt) => {
                    println!("Hello, {name}! (tx: {})", receipt.tx_hash);
                }
                Err(e) => {
                    eprintln!("Payment failed: {e}");
                    process::exit(42);
                }
            }
        }
        "fortune" => {
            let recipient = "0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266";
            match payg::charge("0.002 USDC", recipient).await {
                Ok(receipt) => {
                    println!("Your fortune: The best time to plant a tree was 20 years ago.");
                    println!("(tx: {})", receipt.tx_hash);
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
            eprintln!("Usage: simple-cli <greet|fortune|count-words> [args...]");
            process::exit(1);
        }
    }
}
