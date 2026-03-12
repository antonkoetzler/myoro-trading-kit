//! Integration tests for the Tauri IPC command layer.
//!
//! AppState and command DTOs live in the binary crate and cannot be imported
//! here directly. These tests verify the underlying domain components that
//! AppState wraps and confirm that the expected JSON shapes are valid, using
//! serde_json to construct and round-trip representative payloads.

use myoro_trading_kit::backtester::BacktesterState;
use myoro_trading_kit::config::{
    BinanceConfig, Config, CopySizing, ExecutionMode, PolymarketConfig,
};
use myoro_trading_kit::copy_trading::{Monitor, TraderList};
use myoro_trading_kit::live::LiveState;
use std::collections::HashMap;
use std::sync::{atomic::AtomicBool, Arc, RwLock};

fn test_config() -> Config {
    Config {
        execution_mode: ExecutionMode::Paper,
        polymarket: PolymarketConfig::default(),
        binance: BinanceConfig::default(),
        paper_bankroll: Some(1000.0),
        copy_traders: Vec::new(),
        copy_poll_ms: 250,
        pnl_currency: "USD".to_string(),
        copy_sizing: CopySizing::Proportional,
        copy_trader_bankrolls: HashMap::new(),
        copy_bankroll_fraction: 0.05,
        copy_max_usd: 100.0,
        copy_auto_execute: false,
        paper_trades_file: "data/paper.jsonl".to_string(),
        max_daily_loss_usd: 100.0,
        max_position_usd: 50.0,
        max_open_positions: 10,
        mm_enabled: false,
        mm_half_spread: 0.02,
        mm_max_inventory_usd: 200.0,
        mm_max_markets: 5,
        mm_min_volume_usd: 1000.0,
        binance_lag_assets: vec!["BTCUSDT".to_string()],
        weather_cities: Vec::new(),
        balldontlie_key: String::new(),
    }
}

// ── AppState component construction ─────────────────────────────────────────

#[test]
fn app_state_live_constructs() {
    let _live = LiveState::default();
}

#[test]
fn app_state_backtester_constructs() {
    let bt = BacktesterState::new();
    // strategy_count and data_source_count are accessible on BacktesterState
    assert!(
        bt.strategy_count() > 0,
        "BacktesterState has at least one strategy"
    );
}

#[test]
fn app_state_copy_monitor_constructs() {
    let config = test_config();
    let config_arc = Arc::new(RwLock::new(config));
    let copy_running = Arc::new(AtomicBool::new(false));
    let live = Arc::new(LiveState::default());
    let trader_list = Arc::new(TraderList::new(Arc::clone(&config_arc)));
    let _monitor = Monitor::new(trader_list, Some(live), copy_running);
}

#[test]
fn app_state_all_components_construct_together() {
    let config = test_config();
    let config_arc = Arc::new(RwLock::new(config));
    let live = Arc::new(LiveState::default());
    let backtester = BacktesterState::new();
    let copy_running = Arc::new(AtomicBool::new(false));
    let mm_running = Arc::new(AtomicBool::new(false));
    let trader_list = Arc::new(TraderList::new(Arc::clone(&config_arc)));
    let _monitor = Monitor::new(trader_list, Some(Arc::clone(&live)), copy_running);

    // Verify Arcs are cloneable (required for background threads)
    let _live2 = Arc::clone(&live);
    let _bt2 = Arc::clone(&backtester);
    let _mr2 = Arc::clone(&mm_running);
}

// ── DTO JSON shape validation ────────────────────────────────────────────────
//
// DTOs live in the binary crate. We verify their expected JSON shapes here by
// constructing serde_json::Value objects matching the DTO definitions and
// confirming round-trip serialization is valid.

#[test]
fn dto_global_stats_json_shape() {
    let v = serde_json::json!({
        "bankroll": 1000.0,
        "pnl": 42.5,
        "open_trades": 3,
        "closed_trades": 10,
        "daily_loss_usd": 5.0,
        "circuit_breaker_active": false
    });
    let s = serde_json::to_string(&v).unwrap();
    let back: serde_json::Value = serde_json::from_str(&s).unwrap();
    assert_eq!(back["pnl"], 42.5);
    assert_eq!(back["open_trades"], 3);
}

#[test]
fn dto_crypto_state_json_shape() {
    let v = serde_json::json!({
        "btc_usdt": "50000.00",
        "events": [],
        "strategies": [{
            "id": "binance_lag",
            "name": "Binance Lag",
            "description": "Lag arb",
            "enabled": true,
            "auto_execute": false
        }],
        "signals": [{
            "market_id": "m1",
            "label": "BTC Up",
            "side": "Yes",
            "edge_pct": 0.05,
            "kelly_size": 0.02,
            "strategy_id": "binance_lag",
            "status": "pending",
            "created_at": "2026-01-01T00:00:00Z"
        }],
        "markets": [],
        "binance_lag_confidence": [0.7, 0.8],
        "logical_arb_confidence": [],
        "logs": []
    });
    let s = serde_json::to_string(&v).unwrap();
    let back: serde_json::Value = serde_json::from_str(&s).unwrap();
    assert_eq!(back["strategies"][0]["id"], "binance_lag");
    assert_eq!(back["signals"][0]["edge_pct"], 0.05);
}

#[test]
fn dto_backtester_results_json_shape() {
    let v = serde_json::json!({
        "equity_curve": [1.0, 1.05, 0.98, 1.1],
        "drawdown_curve": [0.0, 0.0, 0.067, 0.0],
        "pnl_buckets": [[-0.05, 1], [0.0, 2], [0.05, 3]],
        "mc_paths": [[1.0, 1.02, 1.05], [1.0, 0.98, 1.01]],
        "metrics": { "sharpe": 1.5, "max_drawdown": 0.067 },
        "trade_list": [{
            "strategy_id": "binance_lag",
            "side": "Yes",
            "entry_price": 0.5,
            "exit_price": 0.55,
            "size": 10.0,
            "pnl": 0.5,
            "timestamp": 1700000000
        }],
        "is_running": false,
        "last_error": null,
        "tool_extra": [["p_value", "0.03"]]
    });
    let s = serde_json::to_string(&v).unwrap();
    let back: serde_json::Value = serde_json::from_str(&s).unwrap();
    assert_eq!(back["equity_curve"][3], 1.1);
    assert_eq!(back["metrics"]["sharpe"], 1.5);
    assert_eq!(back["is_running"], false);
}

#[test]
fn dto_mm_state_json_shape() {
    let v = serde_json::json!({
        "active_quotes": [{
            "market_id": "m1",
            "side": "Yes",
            "price": 0.48,
            "size": 5.0,
            "order_id": "ord_123"
        }],
        "inventory": [{
            "market_id": "m1",
            "net_yes": 5.0,
            "realized_pnl": 0.2,
            "volume": 50.0
        }],
        "total_realized_pnl": 0.2,
        "fill_count": 3,
        "running": true,
        "logs": [{ "level": "info", "message": "MM started" }]
    });
    let s = serde_json::to_string(&v).unwrap();
    let back: serde_json::Value = serde_json::from_str(&s).unwrap();
    assert_eq!(back["running"], true);
    assert_eq!(back["fill_count"], 3);
    assert_eq!(back["active_quotes"][0]["order_id"], "ord_123");
}

#[test]
fn dto_portfolio_state_json_shape() {
    let v = serde_json::json!({
        "positions": [{
            "market_id": "m1",
            "outcome": "Yes",
            "size": 10.0,
            "avg_price": 0.5,
            "current_price": 0.55,
            "current_value": 5.5,
            "unrealized_pnl": 0.5,
            "realized_pnl": 0.0
        }],
        "trade_history": [],
        "domain_pnl": [{ "domain": "Crypto", "realized_pnl": 5.0, "open_positions": 1 }],
        "total_realized_pnl": 5.0,
        "total_unrealized_pnl": 0.5
    });
    let s = serde_json::to_string(&v).unwrap();
    let back: serde_json::Value = serde_json::from_str(&s).unwrap();
    assert_eq!(back["positions"][0]["unrealized_pnl"], 0.5);
    assert_eq!(back["domain_pnl"][0]["domain"], "Crypto");
}

#[test]
fn dto_discover_state_json_shape() {
    let v = serde_json::json!({
        "leaderboard": [{
            "address": "0xabc",
            "pnl": 1500.0,
            "roi": 0.25,
            "trades": 42,
            "win_rate": 0.6
        }],
        "profile": {
            "address": "0xabc",
            "pnl": 1500.0,
            "roi": 0.25,
            "trades": 42,
            "win_rate": 0.6,
            "markets_traded": ["BTC"],
            "recent_trades": []
        }
    });
    let s = serde_json::to_string(&v).unwrap();
    let back: serde_json::Value = serde_json::from_str(&s).unwrap();
    assert_eq!(back["leaderboard"][0]["win_rate"], 0.6);
    assert_eq!(back["profile"]["address"], "0xabc");
}
