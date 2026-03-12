//! Sports tab DTOs.

use serde::Serialize;

#[derive(Serialize, Clone)]
pub struct SportsSignalDto {
    pub market_id: String,
    pub home: String,
    pub away: String,
    pub date: String,
    pub side: String,
    pub edge_pct: f64,
    pub kelly_size: f64,
    pub strategy_id: String,
    pub status: String,
}

#[derive(Serialize, Clone)]
pub struct SportsStrategyDto {
    pub id: String,
    pub name: String,
    pub description: String,
    pub enabled: bool,
    pub auto_execute: bool,
}

#[derive(Serialize, Clone)]
pub struct FixtureDto {
    pub home: String,
    pub away: String,
    pub date: String,
    pub home_xg: f64,
    pub away_xg: f64,
    pub polymarket_id: Option<String>,
}

#[derive(Serialize, Clone)]
pub struct LiveMatchDto {
    pub home_team: String,
    pub away_team: String,
    pub home_goals: u8,
    pub away_goals: u8,
    pub minute: u8,
}

#[derive(Serialize, Clone)]
pub struct SportsStateDto {
    pub strategies: Vec<SportsStrategyDto>,
    pub signals: Vec<SportsSignalDto>,
    pub fixtures: Vec<FixtureDto>,
    pub live_matches: Vec<LiveMatchDto>,
    pub logs: Vec<super::shared::LogEntryDto>,
}
