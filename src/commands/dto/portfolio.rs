//! Portfolio tab DTOs.

use serde::Serialize;

#[derive(Serialize, Clone)]
pub struct PositionDto {
    pub market_id: String,
    pub outcome: String,
    pub size: f64,
    pub avg_price: f64,
    pub current_value: f64,
}

#[derive(Serialize, Clone)]
pub struct TradeRowDto {
    pub timestamp: String,
    pub domain: String,
    pub market_id: String,
    pub side: String,
    pub size: f64,
    pub price: f64,
    pub status: String,
}

#[derive(Serialize, Clone)]
pub struct DomainPnlDto {
    pub domain: String,
    pub today_pnl: f64,
    pub alltime_pnl: f64,
}

#[derive(Serialize, Clone)]
pub struct PortfolioStateDto {
    pub open_positions: Vec<PositionDto>,
    pub trade_history: Vec<TradeRowDto>,
    pub domain_pnl: Vec<DomainPnlDto>,
    pub total_pnl: f64,
}
