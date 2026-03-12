//! Crypto tab DTOs.

use serde::Serialize;

#[derive(Serialize, Clone)]
pub struct CryptoSignalDto {
    pub market_id: String,
    pub label: String,
    pub side: String,
    pub edge_pct: f64,
    pub kelly_size: f64,
    pub strategy_id: String,
    pub status: String,
    pub created_at: String,
}

#[derive(Serialize, Clone)]
pub struct CryptoStrategyDto {
    pub id: String,
    pub name: String,
    pub description: String,
    pub enabled: bool,
    pub auto_execute: bool,
}

#[derive(Serialize, Clone)]
pub struct GammaMarketDto {
    pub id: String,
    pub title: String,
    pub best_bid: f64,
    pub best_ask: f64,
    pub volume: f64,
}

#[derive(Serialize, Clone)]
pub struct CryptoStateDto {
    pub btc_usdt: String,
    pub events: Vec<String>,
    pub strategies: Vec<CryptoStrategyDto>,
    pub signals: Vec<CryptoSignalDto>,
    pub markets: Vec<GammaMarketDto>,
    pub binance_lag_confidence: Vec<f64>,
    pub logical_arb_confidence: Vec<f64>,
    pub logs: Vec<super::shared::LogEntryDto>,
}
