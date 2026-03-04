//! WeatherState and fetch_weather() — Open-Meteo forecast + ForecastLag strategy runner.

use crate::live::global::{push_log_to, LogLevel};
use crate::strategies::weather::{
    CityForecast, ForecastLagStrategy, WeatherSignal, WeatherStrategyConfig,
};
use std::sync::RwLock;

pub struct WeatherState {
    pub forecast: Vec<String>,
    pub strategy_configs: Vec<WeatherStrategyConfig>,
    pub signals: Vec<WeatherSignal>,
    pub city_forecasts: Vec<CityForecast>,
}

impl Default for WeatherState {
    fn default() -> Self {
        Self {
            forecast: Vec::new(),
            strategy_configs: WeatherStrategyConfig::builtins(),
            signals: Vec::new(),
            city_forecasts: Vec::new(),
        }
    }
}

pub fn fetch_weather(
    weather: &RwLock<WeatherState>,
    weather_logs: &RwLock<Vec<(LogLevel, String)>>,
) {
    let meteo = match crate::weather::data::OpenMeteoClient::new() {
        Ok(m) => m,
        Err(_) => {
            push_log_to(weather_logs, LogLevel::Error, "Client init failed".into());
            return;
        }
    };

    push_log_to(
        weather_logs,
        LogLevel::Info,
        "Fetching 7-day forecast (Open-Meteo NYC)…".into(),
    );

    match meteo.fetch_daily(40.7, -74.0) {
        Ok(daily) => {
            let lines: Vec<String> = daily
                .iter()
                .take(7)
                .map(|d| {
                    let max = d
                        .temperature_2m_max
                        .map(|t| t.to_string())
                        .unwrap_or_else(|| "—".to_string());
                    let min = d
                        .temperature_2m_min
                        .map(|t| t.to_string())
                        .unwrap_or_else(|| "—".to_string());
                    format!("{}  max {}°C  min {}°C", d.date, max, min)
                })
                .collect();
            if let Ok(mut w) = weather.write() {
                w.forecast = lines.clone();
            }
            push_log_to(
                weather_logs,
                LogLevel::Success,
                format!("Loaded {} days", lines.len()),
            );
        }
        Err(e) => push_log_to(
            weather_logs,
            LogLevel::Error,
            format!("Open-Meteo fetch failed: {}", e),
        ),
    }

    // ── Run enabled strategies ────────────────────────────────────────────────
    let forecast_lag_enabled = weather
        .read()
        .map(|w| {
            w.strategy_configs
                .iter()
                .any(|s| s.id == "forecast_lag" && s.enabled)
        })
        .unwrap_or(false);

    if forecast_lag_enabled {
        push_log_to(weather_logs, LogLevel::Info, "Running ForecastLag…".into());
        let strategy = ForecastLagStrategy::new();
        match strategy.run() {
            Ok((new_signals, city_forecasts)) => {
                let n = new_signals.len();
                if let Ok(mut w) = weather.write() {
                    w.signals.extend(new_signals);
                    let drain = w.signals.len().saturating_sub(100);
                    if drain > 0 {
                        w.signals.drain(0..drain);
                    }
                    w.city_forecasts = city_forecasts;
                }
                push_log_to(
                    weather_logs,
                    LogLevel::Success,
                    format!("ForecastLag: {} signals", n),
                );
            }
            Err(e) => push_log_to(
                weather_logs,
                LogLevel::Warning,
                format!("ForecastLag error: {}", e),
            ),
        }
    }
}
