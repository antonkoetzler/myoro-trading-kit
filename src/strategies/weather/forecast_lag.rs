//! ForecastLagStrategy: compare NOAA/Open-Meteo temperature forecast against Polymarket odds.
//!
//! Logic:
//!   1. Fetch Open-Meteo daily forecast for NYC, London, Seoul.
//!   2. Fetch Polymarket weather markets from Gamma API (tag: weather).
//!   3. Match city to market by title keywords.
//!   4. Compute implied probability: if forecast max > threshold, P(YES) should be high.
//!   5. Signal when gap between forecast-implied and market odds > MIN_EDGE (6%).

use super::{CityForecast, WeatherSignal, WeatherStrategy};
use crate::shared::strategy::{Signal, StrategyMetadata};
use anyhow::Result;
use chrono::Utc;

const MIN_EDGE: f64 = 0.06;
const KELLY_FRACTION: f64 = 0.25;

/// Cities to monitor with their lat/lon and name keywords.
const CITIES: &[(&str, f64, f64, &[&str])] = &[
    ("New York", 40.7, -74.0, &["new york", "nyc", "ny"]),
    ("London", 51.5, -0.1, &["london", "uk"]),
    ("Seoul", 37.6, 127.0, &["seoul", "korea"]),
];

pub struct ForecastLagStrategy;

impl ForecastLagStrategy {
    pub fn new() -> Self {
        Self
    }

    fn fetch_forecast(lat: f64, lon: f64) -> Result<Option<(f64, f64)>> {
        let meteo = crate::weather::data::OpenMeteoClient::new()?;
        let daily = meteo.fetch_daily(lat, lon)?;
        let today = daily.into_iter().next();
        Ok(today.map(|d| {
            let max = d.temperature_2m_max.unwrap_or(0.0);
            let min = d.temperature_2m_min.unwrap_or(0.0);
            (max, min)
        }))
    }

    fn fetch_weather_markets(client: &reqwest::blocking::Client) -> Result<Vec<serde_json::Value>> {
        let url = "https://gamma-api.polymarket.com/markets?closed=false&limit=50&tag=weather";
        let resp = client.get(url).send()?;
        let arr: Vec<serde_json::Value> = resp.json().unwrap_or_default();
        Ok(arr)
    }

    /// Find the market best matching a city name.
    fn match_market<'a>(
        markets: &'a [serde_json::Value],
        keywords: &[&str],
    ) -> Option<&'a serde_json::Value> {
        markets.iter().find(|m| {
            let title = m
                .get("question")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_lowercase();
            keywords.iter().any(|kw| title.contains(kw))
        })
    }

    /// Extract a temperature threshold from a market title.
    /// Looks for patterns like "> 25°C", "above 30", "exceed 20".
    fn extract_temp_threshold(title: &str) -> Option<f64> {
        let title_lower = title.to_lowercase();
        // Look for patterns: "above X", "> X", "exceed X", "over X"
        for prefix in &["above ", "> ", "exceed ", "over ", "reach "] {
            if let Some(pos) = title_lower.find(prefix) {
                let rest = &title[pos + prefix.len()..];
                let digits: String = rest
                    .chars()
                    .take_while(|c| c.is_ascii_digit() || *c == '.')
                    .collect();
                if let Ok(n) = digits.parse::<f64>() {
                    return Some(n);
                }
            }
        }
        None
    }

    /// Compute market-implied probability given best bid/ask.
    fn market_mid(market: &serde_json::Value) -> f64 {
        let bid = market
            .get("bestBid")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);
        let ask = market
            .get("bestAsk")
            .and_then(|v| v.as_f64())
            .unwrap_or(1.0);
        (bid + ask) / 2.0
    }

    /// Returns (forecast_implied_prob, market_mid, threshold).
    /// forecast_implied_prob = sigmoid-like: if forecast_max > threshold + 5, P ≈ 0.85; etc.
    fn forecast_implied_prob(forecast_max: f64, threshold: f64) -> f64 {
        let gap = forecast_max - threshold;
        // Rough sigmoid: gap of 0 → 0.50, +5 → 0.75, +10 → 0.85, -5 → 0.25
        let prob = 0.5 + gap * 0.05;
        prob.clamp(0.05, 0.95)
    }

    /// Run with config-supplied cities. Falls back to hardcoded CITIES if slice is empty.
    pub fn run_with_cities(
        &self,
        cities: &[(&str, f64, f64, &[&str])],
    ) -> Result<(Vec<WeatherSignal>, Vec<CityForecast>)> {
        if cities.is_empty() {
            return self.run();
        }
        self.run_cities(cities)
    }

    fn run_cities(
        &self,
        cities: &[(&str, f64, f64, &[&str])],
    ) -> Result<(Vec<WeatherSignal>, Vec<CityForecast>)> {
        let client = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(15))
            .build()?;
        let markets = Self::fetch_weather_markets(&client)?;
        let mut signals = Vec::new();
        let mut city_forecasts = Vec::new();

        for (city_name, lat, lon, keywords) in cities {
            let temps = Self::fetch_forecast(*lat, *lon).unwrap_or(None);
            let matched_market = Self::match_market(&markets, keywords);
            let (market_id, market_implied) = matched_market
                .map(|m| {
                    let id = m
                        .get("conditionId")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    let mid = Self::market_mid(m);
                    (Some(id), Some(mid))
                })
                .unwrap_or((None, None));
            city_forecasts.push(CityForecast {
                city: city_name.to_string(),
                today_max_c: temps.map(|(max, _)| max),
                today_min_c: temps.map(|(_, min)| min),
                market_implied,
                market_id: market_id.clone(),
            });
            if let (Some((max, _)), Some(mid), Some(mkt)) = (temps, market_implied, matched_market)
            {
                let title = mkt.get("question").and_then(|v| v.as_str()).unwrap_or("");
                if let Some(threshold) = Self::extract_temp_threshold(title) {
                    let forecast_prob = Self::forecast_implied_prob(max, threshold);
                    let edge = forecast_prob - mid;
                    if edge.abs() >= MIN_EDGE {
                        let (side, edge_pct, kelly) = if edge > 0.0 {
                            let k = (edge / (1.0 - mid)) * KELLY_FRACTION;
                            ("Yes".to_string(), edge, k.min(0.1))
                        } else {
                            let k = (edge.abs() / mid) * KELLY_FRACTION;
                            ("No".to_string(), edge.abs(), k.min(0.1))
                        };
                        signals.push(WeatherSignal {
                            market_id: market_id.clone().unwrap_or_default(),
                            city: city_name.to_string(),
                            label: format!("{} > {:.0}°C", city_name, threshold),
                            side,
                            edge_pct,
                            kelly_size: kelly,
                            strategy_id: "forecast_lag".to_string(),
                            status: "pending".to_string(),
                            created_at: Utc::now(),
                        });
                    }
                }
            }
        }
        Ok((signals, city_forecasts))
    }

    pub fn run(&self) -> Result<(Vec<WeatherSignal>, Vec<CityForecast>)> {
        let client = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(15))
            .build()?;
        let markets = Self::fetch_weather_markets(&client)?;
        let mut signals = Vec::new();
        let mut city_forecasts = Vec::new();

        for (city_name, lat, lon, keywords) in CITIES {
            let temps = Self::fetch_forecast(*lat, *lon).unwrap_or(None);
            let matched_market = Self::match_market(&markets, keywords);

            let (market_id, market_implied) = matched_market
                .map(|m| {
                    let id = m
                        .get("conditionId")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    let mid = Self::market_mid(m);
                    (Some(id), Some(mid))
                })
                .unwrap_or((None, None));

            city_forecasts.push(CityForecast {
                city: city_name.to_string(),
                today_max_c: temps.map(|(max, _)| max),
                today_min_c: temps.map(|(_, min)| min),
                market_implied,
                market_id: market_id.clone(),
            });

            // Generate signal if we have both forecast and market data.
            if let (Some((max, _)), Some(mid), Some(mkt)) = (temps, market_implied, matched_market)
            {
                let title = mkt.get("question").and_then(|v| v.as_str()).unwrap_or("");
                if let Some(threshold) = Self::extract_temp_threshold(title) {
                    let forecast_prob = Self::forecast_implied_prob(max, threshold);
                    let edge = forecast_prob - mid;
                    if edge.abs() >= MIN_EDGE {
                        let (side, edge_pct, kelly) = if edge > 0.0 {
                            let k = (edge / (1.0 - mid)) * KELLY_FRACTION;
                            ("Yes".to_string(), edge, k.min(0.1))
                        } else {
                            let k = (edge.abs() / mid) * KELLY_FRACTION;
                            ("No".to_string(), edge.abs(), k.min(0.1))
                        };
                        signals.push(WeatherSignal {
                            market_id: market_id.clone().unwrap_or_default(),
                            city: city_name.to_string(),
                            label: format!("{} > {:.0}°C", city_name, threshold),
                            side,
                            edge_pct,
                            kelly_size: kelly,
                            strategy_id: "forecast_lag".to_string(),
                            status: "pending".to_string(),
                            created_at: Utc::now(),
                        });
                    }
                }
            }
        }
        Ok((signals, city_forecasts))
    }
}

impl WeatherStrategy for ForecastLagStrategy {
    fn id(&self) -> &'static str {
        "forecast_lag"
    }

    fn metadata(&self) -> StrategyMetadata {
        StrategyMetadata {
            name: "Forecast Lag",
            domain: "weather",
        }
    }

    fn signals(&self) -> Result<Vec<Signal>> {
        Ok(self.run()?.0.into_iter().map(Signal::from).collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_threshold_above_pattern() {
        let t = ForecastLagStrategy::extract_temp_threshold("Will NYC be above 25°C tomorrow?");
        assert_eq!(t, Some(25.0));
    }

    #[test]
    fn extract_threshold_gt_pattern() {
        let t = ForecastLagStrategy::extract_temp_threshold("Temperature > 30 in London?");
        assert_eq!(t, Some(30.0));
    }

    #[test]
    fn extract_threshold_returns_none_for_no_pattern() {
        let t = ForecastLagStrategy::extract_temp_threshold("Will it snow in Seoul?");
        assert_eq!(t, None);
    }

    #[test]
    fn forecast_implied_prob_at_threshold() {
        let p = ForecastLagStrategy::forecast_implied_prob(25.0, 25.0);
        assert!((p - 0.5).abs() < 0.01);
    }

    #[test]
    fn forecast_implied_prob_well_above_threshold() {
        let p = ForecastLagStrategy::forecast_implied_prob(35.0, 25.0);
        assert!(p > 0.75);
    }

    #[test]
    fn forecast_implied_prob_well_below_threshold() {
        let p = ForecastLagStrategy::forecast_implied_prob(15.0, 25.0);
        assert!(p < 0.25);
    }

    #[test]
    fn run_with_cities_falls_back_to_default_when_empty() {
        // Empty slice → run_with_cities delegates to run() without panicking
        let strat = ForecastLagStrategy::new();
        // This calls run() which makes live HTTP calls — just verify no panic on call path.
        // We cannot assert on signals (network-dependent), just check it's callable.
        let _ = strat.run_with_cities(&[]);
    }

    #[test]
    fn run_cities_accepts_custom_city_slice() {
        let strat = ForecastLagStrategy::new();
        // Providing a custom city slice: run_cities is called.
        // No live network in unit tests — but function should be structurally callable.
        let kws: &[&str] = &["london"];
        let cities: &[(&str, f64, f64, &[&str])] = &[("London", 51.5, -0.1, kws)];
        // The function will attempt an HTTP call; we just verify it doesn't panic with valid args.
        let _ = strat.run_with_cities(cities);
    }
}

impl Default for ForecastLagStrategy {
    fn default() -> Self {
        Self::new()
    }
}
