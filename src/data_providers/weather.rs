//! Open-Meteo historical weather data provider.
//! Endpoint: archive-api.open-meteo.com/v1/archive (no auth, global, hourly, back to 1940).
use super::{DataPoint, HistoricalDataProvider, HistoryQuery, TimeSeries};
use crate::strategy_engine::Domain;

pub struct OpenMeteoProvider {
    client: reqwest::blocking::Client,
}

impl Default for OpenMeteoProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl OpenMeteoProvider {
    pub fn new() -> Self {
        Self {
            client: reqwest::blocking::Client::builder()
                .timeout(std::time::Duration::from_secs(15))
                .build()
                .unwrap_or_default(),
        }
    }

    /// Parse "lat,lon" symbol format. Defaults to NYC.
    fn parse_coords(symbol: &str) -> (f64, f64) {
        let parts: Vec<&str> = symbol.split(',').collect();
        if parts.len() == 2 {
            let lat = parts[0].trim().parse().unwrap_or(40.71);
            let lon = parts[1].trim().parse().unwrap_or(-74.01);
            (lat, lon)
        } else {
            // Named locations
            match symbol.to_lowercase().as_str() {
                "nyc" | "new_york" => (40.71, -74.01),
                "london" => (51.51, -0.13),
                "tokyo" => (35.68, 139.69),
                "miami" => (25.76, -80.19),
                "chicago" => (41.88, -87.63),
                _ => (40.71, -74.01),
            }
        }
    }
}

impl HistoricalDataProvider for OpenMeteoProvider {
    fn id(&self) -> &str {
        "open_meteo"
    }

    fn name(&self) -> &str {
        "Weather Data · Open Meteo"
    }

    fn domain(&self) -> Domain {
        Domain::Weather
    }

    fn fetch_history(&self, query: &HistoryQuery) -> anyhow::Result<TimeSeries> {
        let (lat, lon) = Self::parse_coords(&query.symbol);
        let start = chrono::DateTime::from_timestamp(query.start_ts, 0)
            .map(|dt| dt.format("%Y-%m-%d").to_string())
            .unwrap_or_else(|| "2024-01-01".into());
        let end = chrono::DateTime::from_timestamp(query.end_ts, 0)
            .map(|dt| dt.format("%Y-%m-%d").to_string())
            .unwrap_or_else(|| "2024-12-31".into());

        let url = format!(
            "https://archive-api.open-meteo.com/v1/archive?\
            latitude={}&longitude={}&start_date={}&end_date={}\
            &daily=temperature_2m_max,temperature_2m_min,precipitation_sum,wind_speed_10m_max\
            &timezone=auto",
            lat, lon, start, end
        );

        let resp = self.client.get(&url).send()?;
        let body: serde_json::Value = resp.json()?;
        let mut ts = TimeSeries::new("open_meteo", &query.symbol);

        if let Some(daily) = body.get("daily") {
            let times = daily
                .get("time")
                .and_then(|t| t.as_array())
                .cloned()
                .unwrap_or_default();
            let temp_max = extract_f64_array(daily, "temperature_2m_max");
            let temp_min = extract_f64_array(daily, "temperature_2m_min");
            let precip = extract_f64_array(daily, "precipitation_sum");
            let wind = extract_f64_array(daily, "wind_speed_10m_max");

            for (i, time_val) in times.iter().enumerate() {
                let date_str = time_val.as_str().unwrap_or("");
                let timestamp = chrono::NaiveDate::parse_from_str(date_str, "%Y-%m-%d")
                    .map(|d| {
                        d.and_hms_opt(12, 0, 0)
                            .unwrap_or_default()
                            .and_utc()
                            .timestamp()
                    })
                    .unwrap_or(0);

                ts.points.push(DataPoint {
                    timestamp,
                    values: vec![
                        ("temp_max".into(), temp_max.get(i).copied().unwrap_or(0.0)),
                        ("temp_min".into(), temp_min.get(i).copied().unwrap_or(0.0)),
                        ("precip".into(), precip.get(i).copied().unwrap_or(0.0)),
                        ("wind_max".into(), wind.get(i).copied().unwrap_or(0.0)),
                    ],
                });
            }
        }

        Ok(ts)
    }
}

fn extract_f64_array(obj: &serde_json::Value, key: &str) -> Vec<f64> {
    obj.get(key)
        .and_then(|v| v.as_array())
        .map(|arr| arr.iter().map(|v| v.as_f64().unwrap_or(0.0)).collect())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn provider_metadata() {
        let p = OpenMeteoProvider::new();
        assert_eq!(p.id(), "open_meteo");
        assert_eq!(p.domain(), Domain::Weather);
    }

    #[test]
    fn coord_parsing() {
        assert_eq!(
            OpenMeteoProvider::parse_coords("51.51,-0.13"),
            (51.51, -0.13)
        );
        assert_eq!(OpenMeteoProvider::parse_coords("london"), (51.51, -0.13));
        assert_eq!(OpenMeteoProvider::parse_coords("nyc"), (40.71, -74.01));
    }
}
