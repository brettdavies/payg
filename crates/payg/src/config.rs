use std::path::PathBuf;

use alloy_primitives::U256;
use serde::Deserialize;

use crate::error::PaygError;
use crate::network::{self, NetworkConfig};
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
    /// Network name ("base" or "base-sepolia").
    pub network: Option<String>,
}

impl ProjectConfig {
    /// Load `payg.toml` from the current working directory.
    ///
    /// Returns `Ok(None)` if the file does not exist.
    /// Returns `Err` if the file exists but is malformed.
    pub fn load() -> Result<Option<Self>, PaygError> {
        let path = std::env::current_dir()?.join("payg.toml");
        if !path.exists() {
            return Ok(None);
        }
        let contents = std::fs::read_to_string(&path)?;
        let config: Self = toml::from_str(&contents)
            .map_err(|e| PaygError::ConfigError(format!("payg.toml: {e}")))?;
        Ok(Some(config))
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

    /// Facilitator URL, with env var override. Validates HTTPS requirement.
    pub fn facilitator_url(&self) -> Result<String, PaygError> {
        let url = std::env::var("PAYG_FACILITATOR_URL").unwrap_or_else(|_| {
            self.facilitator_url
                .clone()
                .unwrap_or_else(|| DEFAULT_FACILITATOR_URL.to_string())
        });
        validate_url(&url, "facilitator_url")?;
        Ok(url)
    }

    /// Base RPC URL, with env var override. Falls back to network default.
    pub fn rpc_url(&self, network: &NetworkConfig) -> Result<String, PaygError> {
        let url = std::env::var("PAYG_RPC_URL").unwrap_or_else(|_| {
            self.rpc_url
                .clone()
                .unwrap_or_else(|| network.default_rpc_url.to_string())
        });
        validate_url(&url, "rpc_url")?;
        Ok(url)
    }

    /// Resolve the network from: PAYG_NETWORK env var > config field > default (base-sepolia).
    pub fn resolve_network(&self) -> Result<&'static NetworkConfig, PaygError> {
        if let Ok(name) = std::env::var("PAYG_NETWORK") {
            return network::resolve_network_config(&name);
        }
        if let Some(ref name) = self.network {
            return network::resolve_network_config(name);
        }
        Ok(network::DEFAULT_NETWORK)
    }

    /// Parse the safety ceiling into a U256 with its token type.
    ///
    /// Returns (amount in smallest units, token symbol).
    pub fn safety_ceiling_with_token(&self) -> (U256, String) {
        if let Ok(val) = std::env::var("PAYG_MAX_CHARGE")
            && let Ok(parsed) = crate::pricing::parse_price(&val)
        {
            return (parsed.amount, parsed.token);
        }
        if let Some(ref mc) = self.max_charge
            && let Ok(parsed) = crate::pricing::parse_price(mc)
        {
            return (parsed.amount, parsed.token);
        }
        (U256::from(DEFAULT_SAFETY_CEILING_USDC), "USDC".to_string())
    }
}

/// Resolve the user's home directory from the HOME environment variable.
pub fn home_dir() -> Result<PathBuf, PaygError> {
    std::env::var("HOME")
        .map(PathBuf::from)
        .map_err(|_| PaygError::ConfigError("HOME not set".to_string()))
}

/// Validate that a URL uses HTTPS (or HTTP for localhost/127.0.0.1 in dev).
fn validate_url(url: &str, field: &str) -> Result<(), PaygError> {
    if url.starts_with("https://") {
        return Ok(());
    }
    // Allow HTTP for local development only — require exact host boundary
    // to prevent bypass via http://localhost.evil.com or http://127.0.0.1.evil.com
    if is_localhost_url(url) {
        return Ok(());
    }
    Err(PaygError::ConfigError(format!(
        "{field} must use https:// (got: {url})"
    )))
}

/// Check if a URL points to localhost or 127.0.0.1 with exact host boundary.
fn is_localhost_url(url: &str) -> bool {
    for prefix in ["http://localhost", "http://127.0.0.1", "http://[::1]"] {
        if let Some(rest) = url.strip_prefix(prefix) {
            // After the host, only '/', ':', or end-of-string is valid
            if rest.is_empty() || rest.starts_with('/') || rest.starts_with(':') {
                return true;
            }
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_url_accepts_https() {
        assert!(validate_url("https://x402.org/facilitator", "test").is_ok());
    }

    #[test]
    fn validate_url_accepts_localhost() {
        assert!(validate_url("http://localhost:8545", "test").is_ok());
        assert!(validate_url("http://localhost/path", "test").is_ok());
        assert!(validate_url("http://localhost", "test").is_ok());
        assert!(validate_url("http://127.0.0.1:8545", "test").is_ok());
        assert!(validate_url("http://127.0.0.1", "test").is_ok());
        assert!(validate_url("http://[::1]:8545", "test").is_ok());
        assert!(validate_url("http://[::1]", "test").is_ok());
    }

    #[test]
    fn validate_url_rejects_localhost_bypass() {
        assert!(validate_url("http://localhost.evil.com", "test").is_err());
        assert!(validate_url("http://127.0.0.1.evil.com", "test").is_err());
    }

    #[test]
    fn validate_url_rejects_http() {
        assert!(validate_url("http://example.com", "test").is_err());
    }
}
