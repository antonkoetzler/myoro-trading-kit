//! End-to-end integration tests for paper trading and config persistence.
//! These tests exercise real I/O (temp files) but do not require network access.

use myoro_polymarket_terminal::config::{
    BinanceConfig, Config, CopySizing, ExecutionMode, JsonConfigFile, PolymarketConfig,
};
use myoro_polymarket_terminal::live::portfolio::TradeRow;
use myoro_polymarket_terminal::shared::execution::Executor;
use myoro_polymarket_terminal::shared::strategy::Side;
use std::collections::HashMap;
use std::io::Write as _;

fn temp_path(name: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(name)
}

// ── Paper trade write + read back ─────────────────────────────────────────────

#[test]
fn paper_trade_writes_jsonl_and_reads_back() {
    let path = temp_path("e2e_paper_trades.jsonl");
    // Clean up from any previous run
    std::fs::remove_file(&path).ok();

    let exec = Executor::new(ExecutionMode::Paper, path.to_str().unwrap());
    exec.execute_with_price("market-001", Side::Yes, 25.0, 0.60)
        .expect("paper execute should succeed");

    // The file should now exist and contain one valid JSON line
    let content = std::fs::read_to_string(&path).expect("file should exist after paper trade");
    assert!(!content.trim().is_empty(), "file should not be empty");

    let v: serde_json::Value = serde_json::from_str(content.trim()).expect("should be valid JSON");
    assert_eq!(v["market_id"].as_str(), Some("market-001"));
    assert!((v["price"].as_f64().unwrap_or(0.0) - 0.60).abs() < 1e-6);
    assert!((v["size"].as_f64().unwrap_or(0.0) - 25.0).abs() < 1e-6);
    assert!(v["order_id"].as_str().unwrap_or("").starts_with("paper-"));

    std::fs::remove_file(&path).ok();
}

#[test]
fn paper_trade_jsonl_loads_as_trade_row() {
    let path = temp_path("e2e_portfolio_row.jsonl");
    std::fs::remove_file(&path).ok();

    let mut f = std::fs::File::create(&path).unwrap();
    writeln!(
        f,
        r#"{{"timestamp":"2024-06-01T12:00:00Z","domain":"sports","market_id":"mkt-abc","side":"YES","size":10.0,"price":0.55,"status":"filled"}}"#
    )
    .unwrap();
    drop(f);

    let rows = TradeRow::load_from_jsonl(path.to_str().unwrap());
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].market_id, "mkt-abc");
    assert_eq!(rows[0].side, "YES");
    assert!((rows[0].size - 10.0).abs() < 1e-9);
    assert!((rows[0].price - 0.55).abs() < 1e-9);
    assert_eq!(rows[0].status, "filled");

    std::fs::remove_file(&path).ok();
}

#[test]
fn multiple_paper_trades_append_correctly() {
    let path = temp_path("e2e_multi_paper_trades.jsonl");
    std::fs::remove_file(&path).ok();

    let exec = Executor::new(ExecutionMode::Paper, path.to_str().unwrap());
    for i in 0..3 {
        exec.execute_with_price(&format!("market-{}", i), Side::No, 10.0 + i as f64, 0.4)
            .expect("each execute should succeed");
    }

    let content = std::fs::read_to_string(&path).unwrap();
    let lines: Vec<&str> = content.lines().filter(|l| !l.trim().is_empty()).collect();
    assert_eq!(lines.len(), 3, "should have 3 JSONL entries");

    std::fs::remove_file(&path).ok();
}

// ── Config file roundtrip ──────────────────────────────────────────────────────

fn build_test_config() -> Config {
    Config {
        execution_mode: ExecutionMode::Paper,
        polymarket: PolymarketConfig::default(),
        binance: BinanceConfig::default(),
        paper_bankroll: Some(2500.0),
        copy_traders: vec!["0xdeadbeef00000000000000000000000000000001".into()],
        copy_poll_ms: 500,
        pnl_currency: "EUR".into(),
        copy_sizing: CopySizing::Fixed,
        copy_trader_bankrolls: HashMap::new(),
        copy_bankroll_fraction: 0.1,
        copy_max_usd: 200.0,
        copy_auto_execute: true,
        paper_trades_file: "data/paper_e2e.jsonl".into(),
        max_daily_loss_usd: 150.0,
        max_position_usd: 75.0,
        max_open_positions: 8,
        mm_enabled: false,
        mm_half_spread: 0.02,
        mm_max_inventory_usd: 200.0,
        mm_max_markets: 5,
        mm_min_volume_usd: 1000.0,
    }
}

#[test]
fn config_file_roundtrip() {
    let cfg = build_test_config();
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

    let serialized = serde_json::to_string_pretty(&file).expect("serialize");
    let loaded: JsonConfigFile = serde_json::from_str(&serialized).expect("deserialize");

    assert_eq!(loaded.paper_bankroll, Some(2500.0));
    assert_eq!(loaded.pnl_currency.as_deref(), Some("EUR"));
    assert_eq!(loaded.copy_sizing, Some(CopySizing::Fixed));
    assert_eq!(loaded.copy_auto_execute, Some(true));
    assert!((loaded.copy_bankroll_fraction.unwrap_or(0.0) - 0.1).abs() < 1e-9);
    assert_eq!(
        loaded.copy_traders,
        vec!["0xdeadbeef00000000000000000000000000000001"]
    );
    assert_eq!(loaded.max_open_positions, Some(8));
    assert!((loaded.max_daily_loss_usd.unwrap_or(0.0) - 150.0).abs() < 1e-9);
}
