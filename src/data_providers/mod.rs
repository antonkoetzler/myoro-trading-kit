//! Historical data providers + SQLite cache for backtester.
//!
//! Each provider implements `HistoricalDataProvider` and returns `TimeSeries` data.
//! The SQLite cache stores fetched data locally with tiered invalidation.
pub mod binance;
pub mod cache;
pub mod espn;
pub mod import;
pub mod polymarket;
pub mod weather;

use crate::strategy_engine::Domain;

/// Time-indexed data point.
#[derive(Debug, Clone)]
pub struct DataPoint {
    pub timestamp: i64,
    pub values: Vec<(String, f64)>,
}

/// A series of time-indexed data.
#[derive(Debug, Clone)]
pub struct TimeSeries {
    pub source: String,
    pub symbol: String,
    pub points: Vec<DataPoint>,
}

impl TimeSeries {
    pub fn new(source: &str, symbol: &str) -> Self {
        Self {
            source: source.to_string(),
            symbol: symbol.to_string(),
            points: Vec::new(),
        }
    }

    pub fn len(&self) -> usize {
        self.points.len()
    }

    pub fn is_empty(&self) -> bool {
        self.points.is_empty()
    }

    /// Extract a named column as Vec<f64>.
    pub fn column(&self, name: &str) -> Vec<f64> {
        self.points
            .iter()
            .filter_map(|p| p.values.iter().find(|(k, _)| k == name).map(|(_, v)| *v))
            .collect()
    }
}

/// Query parameters for historical data.
#[derive(Debug, Clone)]
pub struct HistoryQuery {
    pub symbol: String,
    pub start_ts: i64,
    pub end_ts: i64,
    pub interval: String,
}

impl HistoryQuery {
    /// Create a query for the last N days.
    pub fn last_days(symbol: &str, days: i64) -> Self {
        let now = chrono::Utc::now().timestamp();
        Self {
            symbol: symbol.to_string(),
            start_ts: now - days * 86400,
            end_ts: now,
            interval: "1d".into(),
        }
    }
}

/// Trait for all historical data providers.
pub trait HistoricalDataProvider: Send + Sync {
    fn id(&self) -> &str;
    fn name(&self) -> &str;
    fn domain(&self) -> Domain;
    fn fetch_history(&self, query: &HistoryQuery) -> anyhow::Result<TimeSeries>;
}

/// Get all available providers.
pub fn all_providers() -> Vec<Box<dyn HistoricalDataProvider>> {
    vec![
        Box::new(polymarket::PolymarketProvider::new()),
        Box::new(binance::BinanceProvider::new()),
        Box::new(espn::EspnProvider::new()),
        Box::new(weather::OpenMeteoProvider::new()),
        Box::new(import::ImportProvider::new()),
    ]
}

/// Provider names for display.
pub fn provider_names() -> Vec<(String, String, Domain)> {
    all_providers()
        .iter()
        .map(|p| (p.id().to_string(), p.name().to_string(), p.domain()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn time_series_column() {
        let mut ts = TimeSeries::new("test", "BTC");
        ts.points.push(DataPoint {
            timestamp: 1000,
            values: vec![("close".into(), 42.0), ("volume".into(), 100.0)],
        });
        ts.points.push(DataPoint {
            timestamp: 2000,
            values: vec![("close".into(), 43.0), ("volume".into(), 110.0)],
        });
        let closes = ts.column("close");
        assert_eq!(closes, vec![42.0, 43.0]);
    }

    #[test]
    fn history_query_last_days() {
        let q = HistoryQuery::last_days("BTC/USDT", 30);
        assert!(q.end_ts - q.start_ts >= 29 * 86400);
    }

    #[test]
    fn all_providers_list() {
        let providers = all_providers();
        assert!(providers.len() >= 5);
    }
}
