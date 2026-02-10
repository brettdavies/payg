mod cli;
mod commands;

use clap::Parser;
use tracing_subscriber::EnvFilter;

use cli::{Cli, Command};
use payg::NetworkConfig;

#[tokio::main(flavor = "current_thread")]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .with_target(false)
        .init();

    let cli = Cli::parse();
    let result = match cli.command {
        Command::Init(args) => commands::init::run(args, cli.output, cli.network.as_deref()).await,
        Command::Charge(args) => {
            commands::charge::run(args, cli.output, cli.network.as_deref()).await
        }
        Command::Address(args) => {
            commands::address::run(args, cli.output, cli.network.as_deref()).await
        }
        Command::Balance(args) => {
            commands::balance::run(args, cli.output, cli.network.as_deref()).await
        }
    };

    if let Err(e) = result {
        let code = exit_code(&e);
        if cli.output == cli::OutputFormat::Json {
            let err = serde_json::json!({
                "status": "error",
                "code": error_code(&e),
                "message": e.to_string(),
            });
            println!("{}", serde_json::to_string(&err).unwrap());
        } else {
            eprintln!("error: {e}");
        }
        std::process::exit(code);
    }
}

/// Resolve network config: CLI flag > env var > config file > default.
pub fn resolve_network(
    cli_network: Option<&str>,
    consumer: &payg::ConsumerConfig,
) -> Result<&'static NetworkConfig, payg::PaygError> {
    match cli_network {
        Some(name) => payg::network::resolve_network_config(name),
        None => consumer.resolve_network(),
    }
}

fn exit_code(err: &payg::PaygError) -> i32 {
    match err {
        payg::PaygError::NoWallet => 77,                    // EX_NOPERM
        payg::PaygError::ConfigError(_) => 78,              // EX_CONFIG
        payg::PaygError::ExceedsSafetyCeiling { .. } => 42, // custom: payment rejected
        payg::PaygError::PaymentFailed(_) => 42,
        payg::PaygError::WalletError(_) => 77,
        _ => 1,
    }
}

fn error_code(err: &payg::PaygError) -> &'static str {
    match err {
        payg::PaygError::NoWallet => "NO_WALLET",
        payg::PaygError::ExceedsSafetyCeiling { .. } => "EXCEEDS_CEILING",
        payg::PaygError::PaymentFailed(_) => "PAYMENT_FAILED",
        payg::PaygError::ConfigError(_) => "CONFIG_ERROR",
        payg::PaygError::WalletError(_) => "WALLET_ERROR",
        payg::PaygError::Io(_) => "IO_ERROR",
        payg::PaygError::Http(_) => "HTTP_ERROR",
        payg::PaygError::Json(_) => "JSON_ERROR",
    }
}
