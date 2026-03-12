//! Copy-trade execution: sizing, side parsing, and paper/live dispatch.

use crate::config;
use crate::copy_trading::fetcher::append_paper_trade_jsonl;
use crate::copy_trading::types::TradeRow;
use crate::shared::execution::Executor;
use crate::shared::strategy::Side;

const MIN_COPY_USD: f64 = 0.01;

fn parse_copy_side(side: &str) -> Option<Side> {
    if side.eq_ignore_ascii_case("buy") || side.eq_ignore_ascii_case("yes") {
        Some(Side::Yes)
    } else if side.eq_ignore_ascii_case("sell") || side.eq_ignore_ascii_case("no") {
        Some(Side::No)
    } else {
        None
    }
}

fn lookup_trader_bankroll(config: &config::Config, trader_addr: &str) -> Option<f64> {
    if let Some(v) = config.copy_trader_bankrolls.get(trader_addr).copied() {
        return Some(v);
    }
    let needle = trader_addr.to_lowercase();
    config
        .copy_trader_bankrolls
        .iter()
        .find_map(|(k, v)| (k.to_lowercase() == needle).then_some(*v))
}

pub fn compute_copy_size(
    config: &config::Config,
    trader_addr: &str,
    trader_size: f64,
) -> Option<f64> {
    if trader_size <= 0.0 {
        return None;
    }
    let my_bankroll = config.paper_bankroll?;
    if my_bankroll <= 0.0 {
        return None;
    }
    let raw = match config.copy_sizing {
        config::CopySizing::Proportional => {
            let trader_bankroll = lookup_trader_bankroll(config, trader_addr)?;
            if trader_bankroll <= 0.0 {
                return None;
            }
            (my_bankroll / trader_bankroll) * trader_size
        }
        config::CopySizing::Fixed => my_bankroll * config.copy_bankroll_fraction,
    };
    let sized = raw.min(config.copy_max_usd.max(MIN_COPY_USD));
    (sized >= MIN_COPY_USD).then_some(sized)
}

/// Runs the execution loop for fetched trades: execute, log, and append paper trades.
pub fn execute_copy_trades(
    trades: &[TradeRow],
    cfg: &config::Config,
    log_sink: Option<&crate::live::LiveState>,
) {
    if !cfg.copy_auto_execute {
        return;
    }
    let exec = Executor::new(cfg.execution_mode, &cfg.paper_trades_file).with_domain("copy");
    for r in trades {
        if r.condition_id.is_none() || r.asset_id.is_none() {
            continue;
        }
        let Some(side) = parse_copy_side(&r.side) else {
            continue;
        };
        let Some(amount) = compute_copy_size(cfg, &r.user, r.size) else {
            if let Some(live) = log_sink {
                live.push_copy_log(
                    crate::live::LogLevel::Warning,
                    format!("Skipped copy trade for {} (invalid sizing inputs)", r.user),
                );
            }
            continue;
        };
        if exec
            .execute(r.condition_id.as_deref().unwrap_or_default(), side, amount)
            .is_err()
        {
            if let Some(live) = log_sink {
                live.push_copy_log(
                    crate::live::LogLevel::Error,
                    format!("Copy execute failed for {}", r.title),
                );
            }
            continue;
        }
        if cfg.execution_mode == config::ExecutionMode::Paper {
            if let Some(live) = log_sink {
                live.push_copy_log(
                    crate::live::LogLevel::Success,
                    format!(
                        "Paper copy: {} {:.4} @ {} · {}",
                        r.side, amount, r.price, r.title
                    ),
                );
            }
            let _ = append_paper_trade_jsonl(&cfg.paper_trades_file, r, amount);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    fn base_config() -> config::Config {
        config::Config {
            execution_mode: config::ExecutionMode::Paper,
            polymarket: config::PolymarketConfig::default(),
            binance: config::BinanceConfig::default(),
            paper_bankroll: Some(10.0),
            copy_traders: vec![],
            copy_poll_ms: 250,
            pnl_currency: "USD".to_string(),
            copy_sizing: config::CopySizing::Proportional,
            copy_trader_bankrolls: std::collections::HashMap::new(),
            copy_bankroll_fraction: 0.05,
            copy_max_usd: 1000.0,
            copy_auto_execute: false,
            paper_trades_file: "data/paper_copy_trades.jsonl".to_string(),
            max_daily_loss_usd: 100.0,
            max_position_usd: 50.0,
            max_open_positions: 10,
            mm_enabled: false,
            mm_half_spread: 0.02,
            mm_max_inventory_usd: 200.0,
            mm_max_markets: 5,
            mm_min_volume_usd: 1000.0,
            binance_lag_assets: vec!["BTCUSDT".to_string()],
            weather_cities: vec![],
            balldontlie_key: String::new(),
        }
    }

    #[test]
    fn copy_size_proportional_and_fixed() {
        let mut cfg = base_config();
        cfg.copy_trader_bankrolls
            .insert("0xabc".to_string(), 10_000.0);
        let proportional = compute_copy_size(&cfg, "0xabc", 1000.0).expect("size");
        assert!((proportional - 1.0).abs() < 1e-9);

        cfg.copy_sizing = config::CopySizing::Fixed;
        cfg.copy_bankroll_fraction = 0.2;
        cfg.copy_max_usd = 1.5;
        let fixed = compute_copy_size(&cfg, "0xabc", 1000.0).expect("fixed");
        assert!((fixed - 1.5).abs() < 1e-9);
    }

    #[test]
    fn execute_copy_trades_with_auto_execute_writes_paper_and_logs() {
        let mut cfg = base_config();
        cfg.copy_auto_execute = true;
        cfg.copy_trader_bankrolls
            .insert("0xuser".to_string(), 1000.0);
        let ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("time")
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("execute_copy_test_{}", ts));
        std::fs::create_dir_all(&dir).expect("mkdir");
        cfg.paper_trades_file = dir.join("trades.jsonl").to_str().expect("path").to_string();

        let live = Arc::new(crate::live::LiveState::default());
        let trade = TradeRow {
            user: "0xuser".to_string(),
            side: "BUY".to_string(),
            size: 10.0,
            price: 0.55,
            title: "Test Market".to_string(),
            outcome: "YES".to_string(),
            ts: 1,
            tx: "0xtx".to_string(),
            condition_id: Some("cond1".to_string()),
            asset_id: Some("asset1".to_string()),
        };

        execute_copy_trades(&[trade], &cfg, Some(live.as_ref()));

        let body = std::fs::read_to_string(&cfg.paper_trades_file).expect("read");
        assert!(body.contains("cond1"));
        assert!(body.contains("Test Market"));

        let logs = live.get_copy_logs();
        let has_paper_copy = logs.iter().any(|(_, msg)| msg.contains("Paper copy:"));
        assert!(
            has_paper_copy,
            "expected log to contain 'Paper copy:', got {:?}",
            logs
        );

        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn execute_copy_trades_without_auto_execute_does_not_write() {
        let mut cfg = base_config();
        cfg.copy_auto_execute = false;
        let ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("time")
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("execute_copy_no_exec_{}", ts));
        std::fs::create_dir_all(&dir).expect("mkdir");
        let file_path = dir.join("trades.jsonl");
        cfg.paper_trades_file = file_path.to_str().expect("path").to_string();

        let trade = TradeRow {
            user: "0xuser".to_string(),
            side: "BUY".to_string(),
            size: 10.0,
            price: 0.5,
            title: "M".to_string(),
            outcome: "YES".to_string(),
            ts: 1,
            tx: "tx".to_string(),
            condition_id: Some("c".to_string()),
            asset_id: Some("a".to_string()),
        };

        execute_copy_trades(&[trade], &cfg, None);

        assert!(
            !file_path.exists(),
            "file should not be created when copy_auto_execute is false"
        );
        let _ = std::fs::remove_dir_all(dir);
    }
}
