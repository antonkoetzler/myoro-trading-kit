//! Crypto domain strategies: BinanceLag, LogicalArb.

pub mod backtest;
pub mod binance_lag;
pub mod data;
pub mod logical_arb;

pub use binance_lag::BinanceLagStrategy;
pub use logical_arb::LogicalArbStrategy;

use crate::shared::strategy::{Signal, StrategyMetadata};
use anyhow::Result;

/// Strategy trait for crypto domain (blocking variant, called from background thread).
pub trait CryptoStrategy: Send + Sync {
    fn id(&self) -> &'static str;
    fn metadata(&self) -> StrategyMetadata;
    /// Returns a signal if edge is found, or None.
    fn signal(&self) -> Result<Option<Signal>>;
}

/// A stored crypto signal with display metadata.
#[derive(Clone, Debug)]
pub struct StoredCryptoSignal {
    pub market_id: String,
    pub label: String,
    pub side: String,
    pub edge_pct: f64,
    pub kelly_size: f64,
    pub strategy_id: String,
    pub status: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// Configuration for a crypto strategy.
#[derive(Clone, Debug)]
pub struct CryptoStrategyConfig {
    pub id: &'static str,
    pub name: &'static str,
    pub description: &'static str,
    pub enabled: bool,
    pub auto_execute: bool,
}

impl CryptoStrategyConfig {
    pub fn builtins() -> Vec<CryptoStrategyConfig> {
        vec![
            CryptoStrategyConfig {
                id: "binance_lag",
                name: "Binance Lag",
                description: "5-min BTC/ETH momentum vs Polymarket. Min edge 3%.",
                enabled: false,
                auto_execute: false,
            },
            CryptoStrategyConfig {
                id: "logical_arb",
                name: "Logical Arb",
                description: "Correlated crypto markets: P(A)<P(B) for nested events.",
                enabled: false,
                auto_execute: false,
            },
        ]
    }
}

/// Gamma market entry for crypto tab display.
#[derive(Clone, Debug)]
pub struct GammaMarket {
    pub id: String,
    pub title: String,
    pub best_bid: f64,
    pub best_ask: f64,
    pub volume: f64,
}
