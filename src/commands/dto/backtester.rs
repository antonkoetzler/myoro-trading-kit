//! Backtester tab DTOs.

use serde::Serialize;
use std::collections::HashMap;

#[derive(Serialize, Clone)]
pub struct ToolParamDto {
    pub name: String,
    pub value: f64,
    pub min: f64,
    pub max: f64,
    pub step: f64,
}

#[derive(Serialize, Clone)]
pub struct ToolDto {
    pub idx: usize,
    pub name: String,
    pub tooltip: String,
    pub params: Vec<ToolParamDto>,
}

#[derive(Serialize, Clone)]
pub struct StrategyEntryDto {
    pub id: String,
    pub name: String,
    pub domain: String,
}

#[derive(Serialize, Clone)]
pub struct DataSourceDto {
    pub id: String,
    pub name: String,
    pub domain: String,
}

#[derive(Serialize, Clone)]
pub struct BacktestTradeRowDto {
    pub strategy_id: String,
    pub side: String,
    pub entry_price: f64,
    pub exit_price: f64,
    pub size: f64,
    pub pnl: f64,
    pub timestamp: i64,
}

#[derive(Serialize, Clone)]
pub struct BacktesterResultsDto {
    pub equity_curve: Vec<f64>,
    pub drawdown_curve: Vec<f64>,
    pub pnl_buckets: Vec<(f64, u32)>,
    pub mc_paths: Option<Vec<Vec<f64>>>,
    pub metrics: HashMap<String, f64>,
    pub trade_list: Vec<BacktestTradeRowDto>,
    pub is_running: bool,
    pub last_error: Option<String>,
    pub tool_extra: Vec<(String, String)>,
}

#[derive(Serialize, Clone)]
pub struct BacktesterStateDto {
    pub strategies: Vec<StrategyEntryDto>,
    pub data_sources: Vec<DataSourceDto>,
    pub tools: Vec<ToolDto>,
    pub is_running: bool,
}
