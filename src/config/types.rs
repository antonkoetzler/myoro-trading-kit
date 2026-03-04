//! Config structs, enums, and helper defaults.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ── Default helpers (used as #[serde(default = "...")] attributes) ─────────

pub(super) fn default_max_daily_loss() -> f64 {
    100.0
}
pub(super) fn default_max_position() -> f64 {
    50.0
}
pub(super) fn default_max_open_positions() -> u32 {
    10
}
pub(super) fn default_mm_half_spread() -> f64 {
    0.02
}
pub(super) fn default_mm_max_inventory() -> f64 {
    200.0
}
pub(super) fn default_mm_max_markets() -> u32 {
    5
}
pub(super) fn default_mm_min_volume() -> f64 {
    1000.0
}

// ── Enums ──────────────────────────────────────────────────────────────────

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum CopySizing {
    #[default]
    Proportional,
    Fixed,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ExecutionMode {
    #[default]
    Paper,
    Live,
}

// ── Sub-configs ────────────────────────────────────────────────────────────

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

// ── Main structs ───────────────────────────────────────────────────────────

/// Fields persisted in config.json (no credentials).
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct JsonConfigFile {
    pub paper_bankroll: Option<f64>,
    pub execution_mode: Option<String>,
    #[serde(default)]
    pub copy_traders: Vec<String>,
    #[serde(default)]
    pub copy_poll_ms: Option<u64>,
    pub pnl_currency: Option<String>,
    pub copy_sizing: Option<CopySizing>,
    #[serde(default)]
    pub copy_trader_bankrolls: HashMap<String, f64>,
    pub copy_bankroll_fraction: Option<f64>,
    pub copy_max_usd: Option<f64>,
    pub copy_auto_execute: Option<bool>,
    pub paper_trades_file: Option<String>,
    pub max_daily_loss_usd: Option<f64>,
    pub max_position_usd: Option<f64>,
    pub max_open_positions: Option<u32>,
    pub mm_enabled: Option<bool>,
    pub mm_half_spread: Option<f64>,
    pub mm_max_inventory_usd: Option<f64>,
    pub mm_max_markets: Option<u32>,
    pub mm_min_volume_usd: Option<f64>,
}

#[derive(Clone, Debug, Default, Deserialize)]
pub struct Config {
    pub execution_mode: ExecutionMode,
    #[serde(default)]
    pub polymarket: PolymarketConfig,
    #[serde(default)]
    pub binance: BinanceConfig,
    #[serde(default)]
    pub paper_bankroll: Option<f64>,
    #[serde(default)]
    pub copy_traders: Vec<String>,
    #[serde(default)]
    pub copy_poll_ms: u64,
    #[serde(default)]
    pub pnl_currency: String,
    #[serde(default)]
    pub copy_sizing: CopySizing,
    #[serde(default)]
    pub copy_trader_bankrolls: HashMap<String, f64>,
    #[serde(default)]
    pub copy_bankroll_fraction: f64,
    #[serde(default)]
    pub copy_max_usd: f64,
    #[serde(default)]
    pub copy_auto_execute: bool,
    #[serde(default)]
    pub paper_trades_file: String,
    // ── Risk limits ───────────────────────────────────────────────────────
    #[serde(default = "default_max_daily_loss")]
    pub max_daily_loss_usd: f64,
    #[serde(default = "default_max_position")]
    pub max_position_usd: f64,
    #[serde(default = "default_max_open_positions")]
    pub max_open_positions: u32,
    // ── Market Making ─────────────────────────────────────────────────────
    #[serde(default)]
    pub mm_enabled: bool,
    #[serde(default = "default_mm_half_spread")]
    pub mm_half_spread: f64,
    #[serde(default = "default_mm_max_inventory")]
    pub mm_max_inventory_usd: f64,
    #[serde(default = "default_mm_max_markets")]
    pub mm_max_markets: u32,
    #[serde(default = "default_mm_min_volume")]
    pub mm_min_volume_usd: f64,
}

// ── Helpers ────────────────────────────────────────────────────────────────

/// Parse EXECUTION_MODE string. Used by load() and by tests.
pub fn parse_execution_mode(s: Option<&str>) -> ExecutionMode {
    match s {
        Some(s) if s.eq_ignore_ascii_case("live") => ExecutionMode::Live,
        _ => ExecutionMode::Paper,
    }
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

    #[test]
    fn json_config_roundtrip_copy_fields() {
        let mut file = JsonConfigFile::default();
        file.copy_sizing = Some(CopySizing::Fixed);
        file.copy_trader_bankrolls
            .insert("0xabc".to_string(), 1000.0);
        file.copy_bankroll_fraction = Some(0.25);
        file.copy_max_usd = Some(22.5);
        file.copy_auto_execute = Some(true);
        file.paper_trades_file = Some("data/custom.jsonl".to_string());
        let s = serde_json::to_string(&file).expect("serialize");
        let parsed: JsonConfigFile = serde_json::from_str(&s).expect("deserialize");
        assert_eq!(parsed.copy_sizing, Some(CopySizing::Fixed));
        assert_eq!(parsed.copy_bankroll_fraction, Some(0.25));
        assert_eq!(parsed.copy_max_usd, Some(22.5));
        assert_eq!(parsed.copy_auto_execute, Some(true));
        assert_eq!(
            parsed.paper_trades_file.as_deref(),
            Some("data/custom.jsonl")
        );
        assert_eq!(
            parsed.copy_trader_bankrolls.get("0xabc").copied(),
            Some(1000.0)
        );
    }

    #[test]
    fn config_struct_defaults_copy_fields() {
        let cfg = Config::default();
        assert_eq!(cfg.copy_sizing, CopySizing::Proportional);
        assert_eq!(cfg.copy_bankroll_fraction, 0.0);
        assert_eq!(cfg.copy_max_usd, 0.0);
        assert!(!cfg.copy_auto_execute);
        assert_eq!(cfg.paper_trades_file, "");
        assert!(cfg.copy_trader_bankrolls.is_empty());
    }
}
