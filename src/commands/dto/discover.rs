//! Discover tab DTOs.

use serde::Serialize;

#[derive(Serialize, Clone)]
pub struct LeaderboardEntryDto {
    pub rank: String,
    pub proxy_wallet: String,
    pub user_name: String,
    pub vol: f64,
    pub pnl: f64,
}

#[derive(Serialize, Clone)]
pub struct TraderProfileDto {
    pub address: String,
    pub trade_count: u32,
    pub top_category: String,
    pub win_rate: Option<f64>,
}
