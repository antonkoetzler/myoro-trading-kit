//! Weather tab commands.

use crate::app_state::AppState;
use crate::commands::dto::shared::LogEntryDto;
use crate::commands::dto::weather::*;
use myoro_trading_kit::live::LogLevel;
use tauri::State;

#[tauri::command]
pub fn get_weather_state(state: State<AppState>) -> WeatherStateDto {
    let logs: Vec<LogEntryDto> = state
        .live
        .get_weather_logs()
        .into_iter()
        .map(|(l, m)| LogEntryDto {
            level: level_str(l).to_string(),
            message: m,
        })
        .collect();

    state
        .live
        .weather
        .read()
        .map(|w| WeatherStateDto {
            forecast: w.forecast.clone(),
            strategies: w
                .strategy_configs
                .iter()
                .map(|c| WeatherStrategyDto {
                    id: c.id.to_string(),
                    name: c.name.to_string(),
                    description: c.description.to_string(),
                    enabled: c.enabled,
                    auto_execute: c.auto_execute,
                })
                .collect(),
            signals: w
                .signals
                .iter()
                .map(|s| WeatherSignalDto {
                    market_id: s.market_id.clone(),
                    city: s.city.clone(),
                    label: s.label.clone(),
                    side: s.side.clone(),
                    edge_pct: s.edge_pct,
                    kelly_size: s.kelly_size,
                    strategy_id: s.strategy_id.clone(),
                    status: s.status.clone(),
                })
                .collect(),
            city_forecasts: w
                .city_forecasts
                .iter()
                .map(|c| CityForecastDto {
                    city: c.city.clone(),
                    today_max_c: c.today_max_c,
                    today_min_c: c.today_min_c,
                    market_implied: c.market_implied,
                    market_id: c.market_id.clone(),
                })
                .collect(),
            logs: logs.clone(),
        })
        .unwrap_or_else(|_| WeatherStateDto {
            forecast: Vec::new(),
            strategies: Vec::new(),
            signals: Vec::new(),
            city_forecasts: Vec::new(),
            logs,
        })
}

#[tauri::command]
pub fn toggle_weather_strategy(state: State<AppState>, idx: usize, enabled: bool) {
    if let Ok(mut w) = state.live.weather.write() {
        if let Some(c) = w.strategy_configs.get_mut(idx) {
            c.enabled = enabled;
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
