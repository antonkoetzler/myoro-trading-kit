//! Binance klines historical data provider.
//! Endpoint: api.binance.com/api/v3/klines (no auth, 6000 weight/min).
use super::{DataPoint, HistoricalDataProvider, HistoryQuery, TimeSeries};
use crate::strategy_engine::Domain;

pub struct BinanceProvider {
    client: reqwest::blocking::Client,
}

impl Default for BinanceProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl BinanceProvider {
    pub fn new() -> Self {
        Self {
            client: reqwest::blocking::Client::builder()
                .timeout(std::time::Duration::from_secs(15))
                .build()
                .unwrap_or_default(),
        }
    }

    fn interval_to_binance(interval: &str) -> &str {
        match interval {
            "1m" => "1m",
            "5m" => "5m",
            "15m" => "15m",
            "1h" => "1h",
            "4h" => "4h",
            "1d" => "1d",
            _ => "1d",
        }
    }
}

impl HistoricalDataProvider for BinanceProvider {
    fn id(&self) -> &str {
        "binance"
    }

    fn name(&self) -> &str {
        "Binance Klines"
    }

    fn domain(&self) -> Domain {
        Domain::Crypto
    }

    fn fetch_history(&self, query: &HistoryQuery) -> anyhow::Result<TimeSeries> {
        let interval = Self::interval_to_binance(&query.interval);
        let url = format!(
            "https://api.binance.com/api/v3/klines?symbol={}&interval={}&startTime={}&endTime={}&limit=1000",
            query.symbol,
            interval,
            query.start_ts * 1000, // Binance uses ms
            query.end_ts * 1000,
        );

        let resp = self.client.get(&url).send()?;
        let body: serde_json::Value = resp.json()?;
        let mut ts = TimeSeries::new("binance", &query.symbol);

        // Binance klines: [[open_time, open, high, low, close, volume, ...], ...]
        if let Some(klines) = body.as_array() {
            for k in klines {
                if let Some(arr) = k.as_array() {
                    let t = arr.first().and_then(|v| v.as_i64()).unwrap_or(0) / 1000;
                    let open = parse_str_f64(arr.get(1));
                    let high = parse_str_f64(arr.get(2));
                    let low = parse_str_f64(arr.get(3));
                    let close = parse_str_f64(arr.get(4));
                    let volume = parse_str_f64(arr.get(5));
                    ts.points.push(DataPoint {
                        timestamp: t,
                        values: vec![
                            ("open".into(), open),
                            ("high".into(), high),
                            ("low".into(), low),
                            ("close".into(), close),
                            ("volume".into(), volume),
                        ],
                    });
                }
            }
        }

        Ok(ts)
    }
}

fn parse_str_f64(v: Option<&serde_json::Value>) -> f64 {
    v.and_then(|x| {
        x.as_f64()
            .or_else(|| x.as_str().and_then(|s| s.parse().ok()))
    })
    .unwrap_or(0.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn provider_metadata() {
        let p = BinanceProvider::new();
        assert_eq!(p.id(), "binance");
        assert_eq!(p.domain(), Domain::Crypto);
    }

    #[test]
    fn interval_mapping() {
        assert_eq!(BinanceProvider::interval_to_binance("1h"), "1h");
        assert_eq!(BinanceProvider::interval_to_binance("1d"), "1d");
        assert_eq!(BinanceProvider::interval_to_binance("unknown"), "1d");
    }
}
