//! Copy trading tab DTOs.

use serde::Serialize;

#[derive(Serialize, Clone)]
pub struct CopyTradeRowDto {
    pub user: String,
    pub side: String,
    pub size: f64,
    pub price: f64,
    pub title: String,
    pub outcome: String,
    pub ts: i64,
    pub tx: String,
}

#[derive(Serialize, Clone)]
pub struct CopyStateDto {
    pub traders: Vec<String>,
    pub is_running: bool,
    pub recent_trades: Vec<CopyTradeRowDto>,
    pub logs: Vec<super::shared::LogEntryDto>,
}
