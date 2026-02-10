use std::path::PathBuf;

use alloy_primitives::U256;
use serde::Deserialize;

use crate::error::PaygError;
use crate::{DEFAULT_FACILITATOR_URL, DEFAULT_SAFETY_CEILING_USDC};

/// Project-level config from `payg.toml` in the current working directory.
///
/// Tool developers create this to define pricing for their CLI tool.
#[derive(Debug, Clone, Deserialize)]
pub struct ProjectConfig {
    /// Recipient wallet address for payments.
    pub recipient: String,
    /// Default price per invocation (e.g. "0.001 USDC").
    pub default_price: Option<String>,
}

/// Consumer-level config from `~/.payg/config.toml`.
///
/// End users configure their wallet and safety settings here.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct ConsumerConfig {
    /// Path to encrypted keyfile (default: ~/.payg/keyfile.json).
    pub keyfile: Option<String>,
    /// Maximum charge per invocation (e.g. "1.00 USDC").
    pub max_charge: Option<String>,
    /// x402 facilitator URL.
    pub facilitator_url: Option<String>,
    /// Base RPC URL for ETH backend and balance queries.
    pub rpc_url: Option<String>,
}

impl ProjectConfig {
    /// Load `payg.toml` from the current working directory.
    pub fn load() -> Result<Self, PaygError> {
        let path = std::env::current_dir()?.join("payg.toml");
        if !path.exists() {
            return Err(PaygError::ConfigError(
                "payg.toml not found in current directory".to_string(),
            ));
        }
        let contents = std::fs::read_to_string(&path)?;
        toml::from_str(&contents).map_err(|e| PaygError::ConfigError(format!("payg.toml: {e}")))
    }
}

impl ConsumerConfig {
    /// Load `~/.payg/config.toml`, falling back to defaults if not found.
    pub fn load() -> Result<Self, PaygError> {
        let path = Self::config_path()?;
        if !path.exists() {
            return Ok(Self::default());
        }
        let contents = std::fs::read_to_string(&path)?;
        toml::from_str(&contents)
            .map_err(|e| PaygError::ConfigError(format!("~/.payg/config.toml: {e}")))
    }

    /// Path to the config file.
    pub fn config_path() -> Result<PathBuf, PaygError> {
        let home = home_dir()?;
        Ok(home.join(".payg").join("config.toml"))
    }

    /// Path to the encrypted keyfile.
    pub fn keyfile_path(&self) -> Result<PathBuf, PaygError> {
        if let Some(ref kf) = self.keyfile {
            let expanded = if let Some(rest) = kf.strip_prefix("~/") {
                home_dir()?.join(rest)
            } else {
                PathBuf::from(kf)
            };
            Ok(expanded)
        } else {
            Ok(home_dir()?.join(".payg").join("keyfile.json"))
        }
    }

    /// Facilitator URL, with env var override.
    pub fn facilitator_url(&self) -> String {
        std::env::var("PAYG_FACILITATOR_URL").unwrap_or_else(|_| {
            self.facilitator_url
                .clone()
                .unwrap_or_else(|| DEFAULT_FACILITATOR_URL.to_string())
        })
    }

    /// Base RPC URL, with env var override.
    pub fn rpc_url(&self) -> String {
        std::env::var("PAYG_RPC_URL").unwrap_or_else(|_| {
            self.rpc_url
                .clone()
                .unwrap_or_else(|| "https://mainnet.base.org".to_string())
        })
    }

    /// Parse the safety ceiling into a U256.
    pub fn safety_ceiling_u256(&self) -> U256 {
        if let Ok(val) = std::env::var("PAYG_MAX_CHARGE")
            && let Ok(parsed) = crate::pricing::parse_price(&val)
        {
            return parsed.amount;
        }
        if let Some(ref mc) = self.max_charge
            && let Ok(parsed) = crate::pricing::parse_price(mc)
        {
            return parsed.amount;
        }
        U256::from(DEFAULT_SAFETY_CEILING_USDC)
    }
}

fn home_dir() -> Result<PathBuf, PaygError> {
    dirs_or_env()
}

fn dirs_or_env() -> Result<PathBuf, PaygError> {
    std::env::var("HOME")
        .map(PathBuf::from)
        .map_err(|_| PaygError::ConfigError("HOME not set".to_string()))
}
