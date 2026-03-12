//! Sports tab commands.

use crate::app_state::AppState;
use crate::commands::dto::shared::LogEntryDto;
use crate::commands::dto::sports::*;
use myoro_trading_kit::live::LogLevel;
use tauri::State;

#[tauri::command]
pub fn get_sports_state(state: State<AppState>) -> SportsStateDto {
    let logs: Vec<LogEntryDto> = state
        .live
        .get_sports_logs()
        .into_iter()
        .map(|(l, m)| LogEntryDto {
            level: level_str(l).to_string(),
            message: m,
        })
        .collect();

    state
        .live
        .sports
        .read()
        .map(|s| SportsStateDto {
            strategies: s
                .strategy_configs
                .iter()
                .map(|c| SportsStrategyDto {
                    id: c.id.to_string(),
                    name: c.name.to_string(),
                    description: c.description.to_string(),
                    enabled: c.enabled,
                    auto_execute: c.auto_execute,
                })
                .collect(),
            signals: s
                .signals
                .iter()
                .map(|sig| SportsSignalDto {
                    market_id: sig.market_id.clone(),
                    home: sig.home.clone(),
                    away: sig.away.clone(),
                    date: sig.date.clone(),
                    side: sig.side.clone(),
                    edge_pct: sig.edge_pct,
                    kelly_size: sig.kelly_size,
                    strategy_id: sig.strategy_id.clone(),
                    status: sig.status.clone(),
                })
                .collect(),
            fixtures: s
                .fixtures
                .iter()
                .map(|f| FixtureDto {
                    home: f.fixture.home.clone(),
                    away: f.fixture.away.clone(),
                    date: f.fixture.date.clone(),
                    home_xg: f.home_xg_per_90,
                    away_xg: f.away_xg_per_90,
                    polymarket_id: f.polymarket.as_ref().map(|p| p.condition_id.clone()),
                })
                .collect(),
            live_matches: s
                .live_matches
                .iter()
                .map(|m| LiveMatchDto {
                    home_team: m.home_team.clone(),
                    away_team: m.away_team.clone(),
                    home_goals: m.home_goals,
                    away_goals: m.away_goals,
                    minute: m.minute,
                })
                .collect(),
            logs: logs.clone(),
        })
        .unwrap_or_else(|_| SportsStateDto {
            strategies: Vec::new(),
            signals: Vec::new(),
            fixtures: Vec::new(),
            live_matches: Vec::new(),
            logs,
        })
}

#[tauri::command]
pub fn toggle_sports_strategy(state: State<AppState>, idx: usize, enabled: bool) {
    if let Ok(mut s) = state.live.sports.write() {
        if let Some(c) = s.strategy_configs.get_mut(idx) {
            c.enabled = enabled;
        }
    }
}

#[tauri::command]
pub fn dismiss_sports_signal(state: State<AppState>, market_id: String) {
    if let Ok(mut s) = state.live.sports.write() {
        if let Some(sig) = s.signals.iter_mut().find(|s| s.market_id == market_id) {
            sig.status = "dismissed".to_string();
        }
    }
}

fn level_str(l: LogLevel) -> &'static str {
    match l {
        LogLevel::Info => "info",
        LogLevel::Success => "success",
        LogLevel::Warning => "warning",
        LogLevel::Error => "error",
    }
}
