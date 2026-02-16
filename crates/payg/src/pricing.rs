use alloy_primitives::U256;

use crate::error::PaygError;

/// A parsed price with amount in token-native units.
#[derive(Debug, Clone)]
pub struct ParsedPrice {
    /// Amount in smallest token unit (e.g. USDC has 6 decimals, ETH has 18).
    pub amount: U256,
    /// Token symbol (e.g. "USDC", "ETH").
    pub token: String,
}

/// Parse a human-readable price string into a `ParsedPrice`.
///
/// Supported formats:
/// - `"0.001 USDC"` -> 1000 (6 decimals)
/// - `"0.01 ETH"` -> 10000000000000000 (18 decimals)
/// - `"free"` -> 0
/// - `"1000"` -> 1000 (raw units, assumed USDC)
pub fn parse_price(input: &str) -> Result<ParsedPrice, PaygError> {
    let input = input.trim();

    if input.eq_ignore_ascii_case("free") {
        return Ok(ParsedPrice {
            amount: U256::ZERO,
            token: "USDC".to_string(),
        });
    }

    let parts: Vec<&str> = input.split_whitespace().collect();
    let (amount_str, token, decimals) = match parts.len() {
        1 => {
            // Raw number, assume USDC
            (parts[0], "USDC".to_string(), 6)
        }
        2 => {
            let token = parts[1].to_uppercase();
            let decimals = match token.as_str() {
                "USDC" => 6,
                "ETH" => 18,
                _ => {
                    return Err(PaygError::ConfigError(format!(
                        "unsupported token: {token}"
                    )));
                }
            };
            (parts[0], token, decimals)
        }
        _ => {
            return Err(PaygError::ConfigError(format!(
                "invalid price format: {input}"
            )));
        }
    };

    let amount = parse_decimal_to_u256(amount_str, decimals)?;

    Ok(ParsedPrice { amount, token })
}

/// Convert a decimal string like "0.001" into U256 with the given number of decimals.
fn parse_decimal_to_u256(s: &str, decimals: u32) -> Result<U256, PaygError> {
    let s = s.trim();
    let multiplier = U256::from(10u64).pow(U256::from(decimals));

    if let Some((whole, frac)) = s.split_once('.') {
        let whole_part: U256 = if whole.is_empty() {
            U256::ZERO
        } else {
            whole
                .parse::<u64>()
                .map(U256::from)
                .map_err(|e| PaygError::ConfigError(format!("invalid amount '{s}': {e}")))?
        };

        // Pad or truncate fractional part to `decimals` digits
        let frac_padded = if frac.len() < decimals as usize {
            format!("{frac:0<width$}", width = decimals as usize)
        } else {
            frac[..decimals as usize].to_string()
        };

        let frac_part: U256 = frac_padded
            .parse::<u64>()
            .map(U256::from)
            .map_err(|e| PaygError::ConfigError(format!("invalid fraction '{s}': {e}")))?;

        Ok(whole_part * multiplier + frac_part)
    } else {
        let whole: U256 = s
            .parse::<u64>()
            .map(U256::from)
            .map_err(|e| PaygError::ConfigError(format!("invalid amount '{s}': {e}")))?;
        Ok(whole * multiplier)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_usdc() {
        let p = parse_price("0.001 USDC").unwrap();
        assert_eq!(p.amount, U256::from(1000u64));
        assert_eq!(p.token, "USDC");
    }

    #[test]
    fn parse_one_usdc() {
        let p = parse_price("1.00 USDC").unwrap();
        assert_eq!(p.amount, U256::from(1_000_000u64));
        assert_eq!(p.token, "USDC");
    }

    #[test]
    fn parse_eth() {
        let p = parse_price("0.01 ETH").unwrap();
        assert_eq!(p.amount, U256::from(10_000_000_000_000_000u64));
        assert_eq!(p.token, "ETH");
    }

    #[test]
    fn parse_free() {
        let p = parse_price("free").unwrap();
        assert_eq!(p.amount, U256::ZERO);
    }

    #[test]
    fn parse_raw_number() {
        let p = parse_price("1000").unwrap();
        // 1000 * 10^6 = 1_000_000_000
        assert_eq!(p.amount, U256::from(1_000_000_000u64));
        assert_eq!(p.token, "USDC");
    }

    #[test]
    fn parse_invalid_token() {
        assert!(parse_price("1.0 BTC").is_err());
    }
}
