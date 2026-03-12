//! Weather domain strategies: ForecastLag (NOAA/Open-Meteo vs Polymarket).

pub mod backtest;
pub mod data;
pub mod forecast_lag;

pub use forecast_lag::ForecastLagStrategy;

use crate::shared::strategy::{Side, Signal, StrategyMetadata};
use anyhow::Result;

/// Strategy trait for weather domain (blocking).
/// `signals()` returns the unified `Signal` type for dispatch/execution.
pub trait WeatherStrategy: Send + Sync {
    fn id(&self) -> &'static str;
    fn metadata(&self) -> StrategyMetadata;
    fn signals(&self) -> Result<Vec<Signal>>;
}

impl From<WeatherSignal> for Signal {
    fn from(w: WeatherSignal) -> Self {
        Signal {
            market_id: w.market_id,
            side: if w.side == "Yes" { Side::Yes } else { Side::No },
            confidence: w.edge_pct.clamp(0.0, 1.0),
            edge_pct: w.edge_pct,
            kelly_size: w.kelly_size,
            auto_execute: false,
            strategy_id: w.strategy_id,
            metadata: None,
            stop_loss_pct: None,
            take_profit_pct: None,
        }
    }
}

/// A stored weather signal.
#[derive(Clone, Debug)]
pub struct WeatherSignal {
    pub market_id: String,
    pub city: String,
    pub label: String,
    pub side: String,
    pub edge_pct: f64,
    pub kelly_size: f64,
    pub strategy_id: String,
    pub status: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// Per-city forecast summary for the Weather tab display.
#[derive(Clone, Debug)]
pub struct CityForecast {
    pub city: String,
    pub today_max_c: Option<f64>,
    pub today_min_c: Option<f64>,
    /// Polymarket implied probability for temperature being above threshold.
    pub market_implied: Option<f64>,
    pub market_id: Option<String>,
}

/// Configuration for a weather strategy.
#[derive(Clone, Debug)]
pub struct WeatherStrategyConfig {
    pub id: &'static str,
    pub name: &'static str,
    pub description: &'static str,
    pub enabled: bool,
    pub auto_execute: bool,
}

impl WeatherStrategyConfig {
    pub fn builtins() -> Vec<WeatherStrategyConfig> {
        vec![WeatherStrategyConfig {
            id: "forecast_lag",
            name: "Forecast Lag",
            description: "NOAA/Open-Meteo vs Polymarket weather markets. Min edge 6%.",
            enabled: false,
            auto_execute: false,
        }]
    }
}
