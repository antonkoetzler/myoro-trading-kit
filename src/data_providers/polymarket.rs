//! Polymarket CLOB historical prices provider.
//! Endpoint: clob.polymarket.com/prices-history (no auth, 1500 req/10s).
use super::{DataPoint, HistoricalDataProvider, HistoryQuery, TimeSeries};
use crate::strategy_engine::Domain;

pub struct PolymarketProvider {
    client: reqwest::blocking::Client,
}

impl Default for PolymarketProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl PolymarketProvider {
    pub fn new() -> Self {
        Self {
            client: reqwest::blocking::Client::builder()
                .timeout(std::time::Duration::from_secs(15))
                .build()
                .unwrap_or_default(),
        }
    }
}

impl HistoricalDataProvider for PolymarketProvider {
    fn id(&self) -> &str {
        "polymarket"
    }

    fn name(&self) -> &str {
        "Polymarket CLOB"
    }

    fn domain(&self) -> Domain {
        Domain::All
    }

    fn fetch_history(&self, query: &HistoryQuery) -> anyhow::Result<TimeSeries> {
        // clob.polymarket.com/prices-history?market={token_id}&interval=1d&fidelity=60
        let url = format!(
            "https://clob.polymarket.com/prices-history?market={}&interval={}&startTs={}&endTs={}",
            query.symbol, query.interval, query.start_ts, query.end_ts
        );

        let resp = self.client.get(&url).send()?;
        let body: serde_json::Value = resp.json()?;

        let mut ts = TimeSeries::new("polymarket", &query.symbol);

        // Response format: {"history": [{"t": timestamp, "p": price}, ...]}
        if let Some(history) = body.get("history").and_then(|h| h.as_array()) {
            for point in history {
                let t = point.get("t").and_then(|v| v.as_i64()).unwrap_or(0);
                let p = point
                    .get("p")
                    .and_then(|v| {
                        v.as_f64()
                            .or_else(|| v.as_str().and_then(|s| s.parse().ok()))
                    })
                    .unwrap_or(0.5);
                ts.points.push(DataPoint {
                    timestamp: t,
                    values: vec![("price".into(), p)],
                });
            }
        }

        Ok(ts)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn provider_metadata() {
        let p = PolymarketProvider::new();
        assert_eq!(p.id(), "polymarket");
        assert_eq!(p.domain(), Domain::All);
    }
}
