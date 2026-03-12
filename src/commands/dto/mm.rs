//! Market making tab DTOs.

use serde::Serialize;

#[derive(Serialize, Clone)]
pub struct ActiveQuoteDto {
    pub market_id: String,
    pub side: String,
    pub price: f64,
    pub size: f64,
    pub order_id: String,
}

#[derive(Serialize, Clone)]
pub struct InventoryEntryDto {
    pub market_id: String,
    pub net_yes: f64,
    pub realized_pnl: f64,
    pub volume: f64,
}

#[derive(Serialize, Clone)]
pub struct MmStateDto {
    pub active_quotes: Vec<ActiveQuoteDto>,
    pub inventory: Vec<InventoryEntryDto>,
    pub total_realized_pnl: f64,
    pub fill_count: u32,
    pub running: bool,
    pub logs: Vec<super::shared::LogEntryDto>,
}
