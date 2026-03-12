//! BallDontLie historical sports data provider.
//! Free API key required — register at https://www.balldontlie.io
//! Covers NBA, NFL, MLB, NHL with official documented endpoints.
use super::{DataPoint, HistoricalDataProvider, HistoryQuery, TimeSeries};
use crate::strategy_engine::Domain;

pub struct BallDontLieProvider {
    client: reqwest::blocking::Client,
    api_key: String,
}

impl Default for BallDontLieProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl BallDontLieProvider {
    pub fn new() -> Self {
        Self {
            client: reqwest::blocking::Client::builder()
                .timeout(std::time::Duration::from_secs(20))
                .build()
                .unwrap_or_default(),
            api_key: std::env::var("BALLDONTLIE_KEY").unwrap_or_default(),
        }
    }

    /// Parse `symbol` like `"nba:games"` → `(sport, endpoint)`.
    fn parse_symbol(symbol: &str) -> (&str, &str) {
        let mut parts = symbol.splitn(2, ':');
        let sport = parts.next().unwrap_or("nba");
        let endpoint = parts.next().unwrap_or("games");
        (sport, endpoint)
    }

    /// Derive season year from timestamp (uses end_ts year).
    fn season_year(ts: i64) -> i32 {
        use chrono::{TimeZone, Utc};
        Utc.timestamp_opt(ts, 0)
            .single()
            .map(|dt| dt.format("%Y").to_string().parse::<i32>().unwrap_or(2024))
            .unwrap_or(2024)
    }
}

impl HistoricalDataProvider for BallDontLieProvider {
    fn id(&self) -> &str {
        "balldontlie"
    }

    fn name(&self) -> &str {
        "Stats · BallDontLie"
    }

    fn domain(&self) -> Domain {
        Domain::Sports
    }

    fn fetch_history(&self, query: &HistoryQuery) -> anyhow::Result<TimeSeries> {
        if self.api_key.is_empty() {
            return Err(anyhow::anyhow!(
                "Set BALLDONTLIE_KEY in .env — register free at balldontlie.io"
            ));
        }

        let (sport, _endpoint) = Self::parse_symbol(&query.symbol);
        let season = Self::season_year(query.end_ts);

        // BallDontLie v1 games endpoint
        let url = format!("https://api.balldontlie.io/v1/{}/games", sport);
        let resp = self
            .client
            .get(&url)
            .header("Authorization", &self.api_key)
            .query(&[
                ("seasons[]", season.to_string()),
                ("per_page", "100".to_string()),
            ])
            .send()?;

        let body: serde_json::Value = resp.json()?;
        let mut ts = TimeSeries::new("balldontlie", &query.symbol);

        // Response: {"data": [{...}, ...]}
        if let Some(games) = body.get("data").and_then(|d| d.as_array()) {
            for game in games {
                let date_str = game.get("date").and_then(|d| d.as_str()).unwrap_or("");
                let timestamp = chrono::DateTime::parse_from_rfc3339(date_str)
                    .map(|dt| dt.timestamp())
                    .unwrap_or_else(|_| {
                        // Try date-only format YYYY-MM-DD
                        chrono::NaiveDate::parse_from_str(date_str, "%Y-%m-%d")
                            .map(|d| {
                                d.and_hms_opt(0, 0, 0)
                                    .map(|dt| dt.and_utc().timestamp())
                                    .unwrap_or(0)
                            })
                            .unwrap_or(0)
                    });

                if timestamp == 0 {
                    continue;
                }
                if timestamp < query.start_ts || timestamp > query.end_ts {
                    continue;
                }

                let home_score = game
                    .get("home_team_score")
                    .and_then(|s| s.as_f64())
                    .unwrap_or(0.0);
                let away_score = game
                    .get("visitor_team_score")
                    .and_then(|s| s.as_f64())
                    .unwrap_or(0.0);

                ts.points.push(DataPoint {
                    timestamp,
                    values: vec![
                        ("home_score".into(), home_score),
                        ("away_score".into(), away_score),
                        ("total".into(), home_score + away_score),
                        ("margin".into(), home_score - away_score),
                    ],
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
        let p = BallDontLieProvider::new();
        assert_eq!(p.id(), "balldontlie");
        assert_eq!(p.name(), "Stats · BallDontLie");
        assert_eq!(p.domain(), Domain::Sports);
    }

    #[test]
    fn symbol_parsing_nba() {
        let (sport, endpoint) = BallDontLieProvider::parse_symbol("nba:games");
        assert_eq!(sport, "nba");
        assert_eq!(endpoint, "games");
    }

    #[test]
    fn symbol_parsing_defaults() {
        let (sport, endpoint) = BallDontLieProvider::parse_symbol("nba");
        assert_eq!(sport, "nba");
        assert_eq!(endpoint, "games");
    }

    #[test]
    fn empty_key_returns_descriptive_error() {
        // Temporarily clear env var for this test
        let p = BallDontLieProvider {
            client: reqwest::blocking::Client::new(),
            api_key: String::new(),
        };
        let q = HistoryQuery::last_days("nba:games", 30);
        let err = p.fetch_history(&q).unwrap_err();
        assert!(
            err.to_string().contains("BALLDONTLIE_KEY"),
            "Error should mention the env var"
        );
    }
}
