//! Weather tab DTOs.

use serde::Serialize;

#[derive(Serialize, Clone)]
pub struct WeatherSignalDto {
    pub market_id: String,
    pub city: String,
    pub label: String,
    pub side: String,
    pub edge_pct: f64,
    pub kelly_size: f64,
    pub strategy_id: String,
    pub status: String,
}

#[derive(Serialize, Clone)]
pub struct WeatherStrategyDto {
    pub id: String,
    pub name: String,
    pub description: String,
    pub enabled: bool,
    pub auto_execute: bool,
}

#[derive(Serialize, Clone)]
pub struct CityForecastDto {
    pub city: String,
    pub today_max_c: Option<f64>,
    pub today_min_c: Option<f64>,
    pub market_implied: Option<f64>,
    pub market_id: Option<String>,
}

#[derive(Serialize, Clone)]
pub struct WeatherStateDto {
    pub forecast: Vec<String>,
    pub strategies: Vec<WeatherStrategyDto>,
    pub signals: Vec<WeatherSignalDto>,
    pub city_forecasts: Vec<CityForecastDto>,
    pub logs: Vec<super::shared::LogEntryDto>,
}
