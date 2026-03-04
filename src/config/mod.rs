#![allow(dead_code)]

pub mod types;
pub use types::*;

use anyhow::Result;
use std::collections::HashMap;

const CONFIG_JSON: &str = "config.json";

pub fn load() -> Result<Config> {
    let _ = dotenvy::dotenv();
    let mut config = Config {
        execution_mode: ExecutionMode::Paper,
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
        paper_bankroll: None,
        copy_traders: Vec::new(),
        copy_poll_ms: 250,
        pnl_currency: "USD".to_string(),
        copy_sizing: CopySizing::Proportional,
        copy_trader_bankrolls: HashMap::new(),
        copy_bankroll_fraction: 0.05,
        copy_max_usd: 1000.0,
        copy_auto_execute: false,
        paper_trades_file: "data/paper_copy_trades.jsonl".to_string(),
        max_daily_loss_usd: types::default_max_daily_loss(),
        max_position_usd: types::default_max_position(),
        max_open_positions: types::default_max_open_positions(),
        mm_enabled: false,
        mm_half_spread: types::default_mm_half_spread(),
        mm_max_inventory_usd: types::default_mm_max_inventory(),
        mm_max_markets: types::default_mm_max_markets(),
        mm_min_volume_usd: types::default_mm_min_volume(),
    };
    if let Ok(data) = std::fs::read_to_string(CONFIG_JSON) {
        if let Ok(file) = serde_json::from_str::<JsonConfigFile>(&data) {
            config.paper_bankroll = file.paper_bankroll.or(config.paper_bankroll);
            config.execution_mode = file
                .execution_mode
                .as_deref()
                .map(|s| parse_execution_mode(Some(s)))
                .unwrap_or(config.execution_mode);
            config.copy_traders = file.copy_traders;
            config.copy_poll_ms = file.copy_poll_ms.unwrap_or(250).clamp(100, 30_000);
            config.copy_sizing = file.copy_sizing.unwrap_or(CopySizing::Proportional);
            config.copy_trader_bankrolls = file.copy_trader_bankrolls;
            let fraction = file.copy_bankroll_fraction.unwrap_or(0.05);
            config.copy_bankroll_fraction = if (0.0..=1.0).contains(&fraction) && fraction > 0.0 {
                fraction
            } else {
                0.05
            };
            config.copy_max_usd = file.copy_max_usd.unwrap_or(1000.0).max(0.01);
            config.copy_auto_execute = file.copy_auto_execute.unwrap_or(false);
            if let Some(path) = file.paper_trades_file {
                if !path.trim().is_empty() {
                    config.paper_trades_file = path;
                }
            }
            if let Some(c) = file.pnl_currency {
                if !c.is_empty() {
                    config.pnl_currency = c;
                }
            }
            if let Some(v) = file.max_daily_loss_usd {
                config.max_daily_loss_usd = v.max(0.0);
            }
            if let Some(v) = file.max_position_usd {
                config.max_position_usd = v.max(0.0);
            }
            if let Some(v) = file.max_open_positions {
                config.max_open_positions = v;
            }
            if let Some(v) = file.mm_enabled {
                config.mm_enabled = v;
            }
            if let Some(v) = file.mm_half_spread {
                config.mm_half_spread = v.clamp(0.001, 0.5);
            }
            if let Some(v) = file.mm_max_inventory_usd {
                config.mm_max_inventory_usd = v.max(0.0);
            }
            if let Some(v) = file.mm_max_markets {
                config.mm_max_markets = v;
            }
            if let Some(v) = file.mm_min_volume_usd {
                config.mm_min_volume_usd = v.max(0.0);
            }
        }
    }
    Ok(config)
}

/// Risk check: returns Err if execution would violate configured limits.
pub fn check_risk(config: &Config, open_positions: u32, daily_loss_so_far: f64) -> Result<()> {
    use anyhow::bail;
    if open_positions >= config.max_open_positions {
        bail!(
            "Risk: max open positions {} reached",
            config.max_open_positions
        );
    }
    if daily_loss_so_far >= config.max_daily_loss_usd {
        bail!(
            "Risk: daily loss limit ${:.2} reached",
            config.max_daily_loss_usd
        );
    }
    Ok(())
}

/// Save dynamic settings to config.json (no credentials).
pub fn save_config(c: &Config) -> Result<()> {
    let file = JsonConfigFile {
        paper_bankroll: c.paper_bankroll,
        execution_mode: Some(
            match c.execution_mode {
                ExecutionMode::Paper => "paper",
                ExecutionMode::Live => "live",
            }
            .to_string(),
        ),
        copy_traders: c.copy_traders.clone(),
        copy_poll_ms: Some(c.copy_poll_ms),
        pnl_currency: Some(c.pnl_currency.clone()),
        copy_sizing: Some(c.copy_sizing),
        copy_trader_bankrolls: c.copy_trader_bankrolls.clone(),
        copy_bankroll_fraction: Some(c.copy_bankroll_fraction),
        copy_max_usd: Some(c.copy_max_usd),
        copy_auto_execute: Some(c.copy_auto_execute),
        paper_trades_file: Some(c.paper_trades_file.clone()),
        max_daily_loss_usd: Some(c.max_daily_loss_usd),
        max_position_usd: Some(c.max_position_usd),
        max_open_positions: Some(c.max_open_positions),
        mm_enabled: Some(c.mm_enabled),
        mm_half_spread: Some(c.mm_half_spread),
        mm_max_inventory_usd: Some(c.mm_max_inventory_usd),
        mm_max_markets: Some(c.mm_max_markets),
        mm_min_volume_usd: Some(c.mm_min_volume_usd),
    };
    let s = serde_json::to_string_pretty(&file)?;
    std::fs::write(CONFIG_JSON, s)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn default_config() -> Config {
        Config {
            execution_mode: ExecutionMode::Paper,
            polymarket: PolymarketConfig::default(),
            binance: BinanceConfig::default(),
            paper_bankroll: Some(1000.0),
            copy_traders: vec!["0xabc".into()],
            copy_poll_ms: 250,
            pnl_currency: "USD".into(),
            copy_sizing: CopySizing::Proportional,
            copy_trader_bankrolls: HashMap::new(),
            copy_bankroll_fraction: 0.05,
            copy_max_usd: 100.0,
            copy_auto_execute: false,
            paper_trades_file: "data/paper.jsonl".into(),
            max_daily_loss_usd: types::default_max_daily_loss(),
            max_position_usd: types::default_max_position(),
            max_open_positions: types::default_max_open_positions(),
            mm_enabled: false,
            mm_half_spread: types::default_mm_half_spread(),
            mm_max_inventory_usd: types::default_mm_max_inventory(),
            mm_max_markets: types::default_mm_max_markets(),
            mm_min_volume_usd: types::default_mm_min_volume(),
        }
    }

    #[test]
    fn check_risk_passes_when_under_limits() {
        let cfg = default_config(); // max_daily_loss=100, max_open_positions=10
        assert!(check_risk(&cfg, 5, 50.0).is_ok());
    }

    #[test]
    fn check_risk_fails_when_positions_at_limit() {
        let cfg = default_config();
        let result = check_risk(&cfg, cfg.max_open_positions, 0.0);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("max open positions"));
    }

    #[test]
    fn check_risk_fails_when_daily_loss_at_limit() {
        let cfg = default_config();
        let result = check_risk(&cfg, 0, cfg.max_daily_loss_usd);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("daily loss"));
    }

    #[test]
    fn save_and_reload_config_roundtrip() {
        let dir = std::env::temp_dir();
        let path = dir.join("test_config_roundtrip.json");

        let cfg = default_config();
        let file = JsonConfigFile {
            paper_bankroll: cfg.paper_bankroll,
            execution_mode: Some("paper".into()),
            copy_traders: cfg.copy_traders.clone(),
            copy_poll_ms: Some(cfg.copy_poll_ms),
            pnl_currency: Some(cfg.pnl_currency.clone()),
            copy_sizing: Some(cfg.copy_sizing),
            copy_trader_bankrolls: cfg.copy_trader_bankrolls.clone(),
            copy_bankroll_fraction: Some(cfg.copy_bankroll_fraction),
            copy_max_usd: Some(cfg.copy_max_usd),
            copy_auto_execute: Some(cfg.copy_auto_execute),
            paper_trades_file: Some(cfg.paper_trades_file.clone()),
            max_daily_loss_usd: Some(cfg.max_daily_loss_usd),
            max_position_usd: Some(cfg.max_position_usd),
            max_open_positions: Some(cfg.max_open_positions),
            mm_enabled: Some(cfg.mm_enabled),
            mm_half_spread: Some(cfg.mm_half_spread),
            mm_max_inventory_usd: Some(cfg.mm_max_inventory_usd),
            mm_max_markets: Some(cfg.mm_max_markets),
            mm_min_volume_usd: Some(cfg.mm_min_volume_usd),
        };
        let s = serde_json::to_string_pretty(&file).unwrap();
        std::fs::write(&path, &s).unwrap();

        let loaded: JsonConfigFile = serde_json::from_str(&s).unwrap();
        assert_eq!(loaded.paper_bankroll, Some(1000.0));
        assert_eq!(loaded.copy_traders, vec!["0xabc"]);
        assert_eq!(loaded.execution_mode.as_deref(), Some("paper"));

        std::fs::remove_file(&path).ok();
    }
}
