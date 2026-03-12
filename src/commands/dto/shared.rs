//! Shared DTO types used across multiple command domains.

use serde::Serialize;

#[derive(Serialize, Clone)]
pub struct LogEntryDto {
    pub level: String,
    pub message: String,
}

#[derive(Serialize, Clone)]
pub struct GlobalStatsDto {
    pub bankroll: Option<f64>,
    pub pnl: f64,
    pub open_trades: u32,
    pub closed_trades: u32,
    pub daily_loss_usd: f64,
    pub circuit_breaker_active: bool,
}

#[derive(Serialize, Clone)]
pub struct ConfigDto {
    pub execution_mode: String,
    pub paper_bankroll: Option<f64>,
    pub pnl_currency: String,
    pub copy_traders: Vec<String>,
    pub copy_poll_ms: u64,
    pub copy_bankroll_fraction: f64,
    pub copy_max_usd: f64,
    pub copy_auto_execute: bool,
    pub max_daily_loss_usd: f64,
    pub max_position_usd: f64,
    pub max_open_positions: u32,
    pub mm_enabled: bool,
    pub mm_half_spread: f64,
    pub mm_max_inventory_usd: f64,
    pub mm_max_markets: u32,
    pub mm_min_volume_usd: f64,
    pub binance_lag_assets: Vec<String>,
}
