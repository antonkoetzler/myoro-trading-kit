//! Unified signals command (across all domains).

use crate::app_state::AppState;
use crate::commands::dto::crypto::CryptoSignalDto;
use crate::commands::dto::sports::SportsSignalDto;
use crate::commands::dto::weather::WeatherSignalDto;
use serde::Serialize;
use tauri::State;

#[derive(Serialize, Clone)]
pub struct AllSignalsDto {
    pub crypto: Vec<CryptoSignalDto>,
    pub sports: Vec<SportsSignalDto>,
    pub weather: Vec<WeatherSignalDto>,
}

#[tauri::command]
pub fn get_all_signals(state: State<AppState>) -> AllSignalsDto {
    let crypto = state
        .live
        .crypto
        .read()
        .map(|c| {
            c.signals
                .iter()
                .filter(|s| s.status == "pending")
                .map(|s| CryptoSignalDto {
                    market_id: s.market_id.clone(),
                    label: s.label.clone(),
                    side: s.side.clone(),
                    edge_pct: s.edge_pct,
                    kelly_size: s.kelly_size,
                    strategy_id: s.strategy_id.clone(),
                    status: s.status.clone(),
                    created_at: s.created_at.to_rfc3339(),
                })
                .collect()
        })
        .unwrap_or_default();

    let sports = state
        .live
        .sports
        .read()
        .map(|s| {
            s.signals
                .iter()
                .filter(|sig| sig.status == "pending")
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
                .collect()
        })
        .unwrap_or_default();

    let weather = state
        .live
        .weather
        .read()
        .map(|w| {
            w.signals
                .iter()
                .filter(|s| s.status == "pending")
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
                .collect()
        })
        .unwrap_or_default();

    AllSignalsDto {
        crypto,
        sports,
        weather,
    }
}
