/// Errors that can occur during PAYG operations.
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
    Http(String),

    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
}
