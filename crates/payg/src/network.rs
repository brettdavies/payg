use crate::error::PaygError;

/// Network-specific configuration for Base L2 chains.
///
/// Bundles the values that differ between Base mainnet and Base Sepolia testnet.
/// Use [`resolve_network_config`] to look up a preset by name.
#[derive(Debug, Clone, Copy)]
pub struct NetworkConfig {
    pub chain_id: u64,
    pub usdc_address: &'static str,
    pub eip712_name: &'static str,
    pub eip712_version: &'static str,
    /// Slug identifier used in config, CLI, and x402 protocol (e.g. "base-sepolia").
    pub name: &'static str,
    /// Human-readable name for CLI output (e.g. "Base Sepolia").
    pub display_name: &'static str,
    pub default_rpc_url: &'static str,
}

pub const BASE_MAINNET: NetworkConfig = NetworkConfig {
    chain_id: 8453,
    usdc_address: "0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913",
    eip712_name: "USD Coin",
    eip712_version: "2",
    name: "base",
    display_name: "Base",
    default_rpc_url: "https://mainnet.base.org",
};

pub const BASE_SEPOLIA: NetworkConfig = NetworkConfig {
    chain_id: 84532,
    usdc_address: "0x036CbD53842c5426634e7929541eC2318f3dCF7e",
    eip712_name: "USDC",
    eip712_version: "2",
    name: "base-sepolia",
    display_name: "Base Sepolia",
    default_rpc_url: "https://sepolia.base.org",
};

pub const DEFAULT_NETWORK: &NetworkConfig = &BASE_SEPOLIA;

/// Resolve a network name to its config. Returns error for unknown networks.
pub fn resolve_network_config(name: &str) -> Result<&'static NetworkConfig, PaygError> {
    match name {
        "base" => Ok(&BASE_MAINNET),
        "base-sepolia" => Ok(&BASE_SEPOLIA),
        _ => Err(PaygError::ConfigError(format!(
            "unknown network '{name}' — valid options: base, base-sepolia"
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_known_networks() {
        let base = resolve_network_config("base").unwrap();
        assert_eq!(base.chain_id, 8453);
        assert_eq!(base.eip712_name, "USD Coin");

        let sepolia = resolve_network_config("base-sepolia").unwrap();
        assert_eq!(sepolia.chain_id, 84532);
        assert_eq!(sepolia.eip712_name, "USDC");
    }

    #[test]
    fn resolve_unknown_network_errors() {
        let err = resolve_network_config("ethereum").unwrap_err();
        assert!(err.to_string().contains("unknown network"));
        assert!(err.to_string().contains("base, base-sepolia"));
    }

    #[test]
    fn default_is_testnet() {
        assert_eq!(DEFAULT_NETWORK.name, "base-sepolia");
    }
}
