//! Core data types for the backtester: trades, strategies, data sources, tool params.
use crate::shared::strategy::Side;
use rand::rngs::SmallRng;
use rand::{Rng, SeedableRng};
use serde::Deserialize;
use std::path::Path;

/// A completed trade (resolved or synthetic).
#[derive(Debug, Clone)]
pub struct Trade {
    pub strategy_id: String,
    pub side: Side,
    pub entry_price: f64,
    pub exit_price: f64,
    pub size: f64,
    pub pnl: f64,
    pub timestamp: i64,
}

/// Descriptor for a selectable strategy in the backtester.
#[derive(Debug, Clone)]
pub struct StrategyEntry {
    pub id: String,
    pub name: String,
    pub domain: String,
}

/// Available data source.
#[derive(Debug, Clone)]
pub struct DataSourceEntry {
    pub id: String,
    pub name: String,
    /// Domain compatibility: "all", "crypto", "sports", "weather"
    pub domain: String,
}

/// Adjustable tool parameter.
#[derive(Debug, Clone)]
pub struct ToolParam {
    pub name: String,
    pub value: f64,
    pub min: f64,
    pub max: f64,
    pub step: f64,
}

impl ToolParam {
    pub fn new(name: &str, value: f64, min: f64, max: f64, step: f64) -> Self {
        Self {
            name: name.to_string(),
            value,
            min,
            max,
            step,
        }
    }

    /// Increment value by one step, clamped to max.
    pub fn increment(&mut self) {
        self.value = (self.value + self.step).min(self.max);
    }

    /// Decrement value by one step, clamped to min.
    pub fn decrement(&mut self) {
        self.value = (self.value - self.step).max(self.min);
    }
}

// ── Default registries ──────────────────────────────────────────────────────

pub fn default_strategies() -> Vec<StrategyEntry> {
    let s = |id: &str, name: &str, domain: &str| StrategyEntry {
        id: id.into(),
        name: name.into(),
        domain: domain.into(),
    };
    vec![
        s("all", "All Strategies", "all"),
        s("binance_lag", "Binance Lag", "crypto"),
        s("logical_arb", "Logical Arb", "crypto"),
        s("poisson", "Poisson", "sports"),
        s("home_advantage", "Home Advantage", "sports"),
        s("rule_1_20", "Rule 1/20", "sports"),
        s("arb_scanner", "Arb Scanner", "sports"),
        s("in_play_70min", "In-Play 70min", "sports"),
        s("forecast_lag", "Forecast Lag", "weather"),
    ]
}

pub fn default_data_sources() -> Vec<DataSourceEntry> {
    let mut sources = vec![
        DataSourceEntry {
            id: "paper".into(),
            name: "Paper Trades".into(),
            domain: "all".into(),
        },
        DataSourceEntry {
            id: "synthetic".into(),
            name: "Synthetic".into(),
            domain: "all".into(),
        },
    ];
    for (id, name, domain) in crate::data_providers::provider_names() {
        sources.push(DataSourceEntry {
            id,
            name,
            domain: domain.as_str().to_string(),
        });
    }
    sources
}

/// Load strategies from the TOML registry + built-in defaults.
pub fn load_strategies_with_registry() -> Vec<StrategyEntry> {
    let mut entries = default_strategies();

    // Load custom TOML strategies from strategies/ directory
    let mut registry = crate::strategy_engine::registry::StrategyRegistry::new();
    registry.load_all_dirs("strategies");

    for (id, name, domain) in registry.names() {
        entries.push(StrategyEntry {
            id,
            name: format!("{} (TOML)", name),
            domain: domain.as_str().to_string(),
        });
    }

    entries
}

// ── Trade loaders ───────────────────────────────────────────────────────────

#[derive(Deserialize)]
struct PaperRecord {
    #[serde(alias = "confidence")]
    predicted: Option<f64>,
    outcome: Option<bool>,
    #[serde(default)]
    strategy_id: String,
}

/// Load trades from paper_trades.jsonl, optionally filtering by strategy.
pub fn load_paper_trades(path: &str, strategy_filter: &str) -> Vec<Trade> {
    let p = Path::new(path);
    if !p.exists() {
        return Vec::new();
    }
    let content = match std::fs::read_to_string(p) {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };
    let mut trades = Vec::new();
    for (i, line) in content.lines().enumerate() {
        let rec: PaperRecord = match serde_json::from_str(line) {
            Ok(r) => r,
            Err(_) => continue,
        };
        if strategy_filter != "all" && rec.strategy_id != strategy_filter {
            continue;
        }
        let predicted = match rec.predicted {
            Some(p) if p > 0.0 && p < 1.0 => p,
            _ => continue,
        };
        let (exit, side) = match rec.outcome {
            Some(true) => (1.0, Side::Yes),
            Some(false) => (0.0, Side::Yes),
            None => continue,
        };
        let pnl = (exit - predicted) * 100.0;
        trades.push(Trade {
            strategy_id: rec.strategy_id.clone(),
            side,
            entry_price: predicted,
            exit_price: exit,
            size: 100.0,
            pnl,
            timestamp: i as i64,
        });
    }
    trades
}

/// Generate synthetic trades for testing.
pub fn generate_synthetic(n: usize, win_rate: f64, avg_win: f64, avg_loss: f64) -> Vec<Trade> {
    let mut rng = SmallRng::seed_from_u64(42);
    (0..n)
        .map(|i| {
            let win = rng.gen::<f64>() < win_rate;
            let pnl = if win {
                avg_win * (0.5 + rng.gen::<f64>())
            } else {
                -avg_loss * (0.5 + rng.gen::<f64>())
            };
            let entry = 0.5;
            let exit = entry + pnl / 100.0;
            Trade {
                strategy_id: "synthetic".into(),
                side: if win { Side::Yes } else { Side::No },
                entry_price: entry,
                exit_price: exit.clamp(0.0, 1.0),
                size: 100.0,
                pnl,
                timestamp: i as i64,
            }
        })
        .collect()
}

/// Build equity curve from trades.
pub fn equity_curve(trades: &[Trade]) -> Vec<f64> {
    let mut curve = Vec::with_capacity(trades.len() + 1);
    curve.push(0.0);
    let mut cum = 0.0;
    for t in trades {
        cum += t.pnl;
        curve.push(cum);
    }
    curve
}
