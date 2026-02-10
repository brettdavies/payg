mod cli;
mod commands;

use clap::Parser;
use tracing_subscriber::EnvFilter;

use cli::{Cli, Command};

#[tokio::main(flavor = "current_thread")]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .with_target(false)
        .init();

    let cli = Cli::parse();
    let result = match cli.command {
        Command::Init(args) => commands::init::run(args).await,
        Command::Charge(args) => commands::charge::run(args, cli.output).await,
        Command::Address(args) => commands::address::run(args, cli.output).await,
        Command::Balance(args) => commands::balance::run(args, cli.output).await,
    };

    if let Err(e) = result {
        let code = exit_code(&e);
        if cli.output == cli::OutputFormat::Json {
            let err = serde_json::json!({
                "status": "error",
                "code": error_code(&e),
                "message": e.to_string(),
            });
            eprintln!("{}", serde_json::to_string(&err).unwrap());
        } else {
            eprintln!("error: {e}");
        }
        std::process::exit(code);
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
