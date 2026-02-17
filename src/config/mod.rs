#![allow(dead_code)]

use anyhow::Result;
use serde::Deserialize;

#[derive(Clone, Debug, Default, Deserialize)]
pub struct Config {
    pub execution_mode: ExecutionMode,
    #[serde(default)]
    pub polymarket: PolymarketConfig,
    #[serde(default)]
    pub binance: BinanceConfig,
    /// Paper-trading bankroll (display; set PAPER_BANKROLL in .env or in-app).
    #[serde(default)]
    pub paper_bankroll: Option<f64>,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ExecutionMode {
    #[default]
    Paper,
    Live,
}

#[derive(Clone, Debug, Default, Deserialize)]
pub struct PolymarketConfig {
    /// Proxy (Safe) address so CLOB orders show under your Polymarket profile.
    pub funder_address: Option<String>,
    pub private_key: Option<String>,
    pub api_key: Option<String>,
    pub api_secret: Option<String>,
    pub api_passphrase: Option<String>,
}

#[derive(Clone, Debug, Default, Deserialize)]
pub struct BinanceConfig {
    pub api_key: Option<String>,
}

/// Parse EXECUTION_MODE string. Used by load() and by tests.
pub fn parse_execution_mode(s: Option<&str>) -> ExecutionMode {
    match s {
        Some(s) if s.eq_ignore_ascii_case("live") => ExecutionMode::Live,
        _ => ExecutionMode::Paper,
    }
}

pub fn load() -> Result<Config> {
    let _ = dotenvy::dotenv();
    let mode = parse_execution_mode(std::env::var("EXECUTION_MODE").ok().as_deref());
    let config = Config {
        execution_mode: mode,
        polymarket: PolymarketConfig {
            funder_address: std::env::var("FUNDER_ADDRESS").ok(),
            private_key: std::env::var("PRIVATE_KEY").ok(),
            api_key: std::env::var("API_KEY").ok(),
            api_secret: std::env::var("API_SECRET").ok(),
            api_passphrase: std::env::var("API_PASSPHRASE").ok(),
        },
        binance: BinanceConfig {
            api_key: std::env::var("BINANCE_API_KEY").ok(),
        },
        paper_bankroll: std::env::var("PAPER_BANKROLL")
            .ok()
            .and_then(|s| s.parse().ok()),
    };
    Ok(config)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn execution_mode_default_is_paper() {
        assert_eq!(parse_execution_mode(None), ExecutionMode::Paper);
    }

    #[test]
    fn execution_mode_parse_live() {
        assert_eq!(parse_execution_mode(Some("live")), ExecutionMode::Live);
        assert_eq!(parse_execution_mode(Some("LIVE")), ExecutionMode::Live);
    }

    #[test]
    fn execution_mode_parse_non_live_is_paper() {
        assert_eq!(parse_execution_mode(Some("paper")), ExecutionMode::Paper);
        assert_eq!(parse_execution_mode(Some("")), ExecutionMode::Paper);
        assert_eq!(parse_execution_mode(Some("other")), ExecutionMode::Paper);
    }
}
