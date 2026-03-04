//! ESPN Sports API historical data provider.
//! Covers NFL, NBA, MLB, NHL, Soccer (EPL, La Liga, etc.), Tennis, MMA.
//! Endpoint: site.api.espn.com/apis/site/v2/sports/{sport}/{league} (no auth).
use super::{DataPoint, HistoricalDataProvider, HistoryQuery, TimeSeries};
use crate::strategy_engine::Domain;

pub struct EspnProvider {
    client: reqwest::blocking::Client,
}

impl Default for EspnProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl EspnProvider {
    pub fn new() -> Self {
        Self {
            client: reqwest::blocking::Client::builder()
                .timeout(std::time::Duration::from_secs(15))
                .build()
                .unwrap_or_default(),
        }
    }

    /// Map a symbol to an ESPN sport/league path.
    fn resolve_league(symbol: &str) -> (&'static str, &'static str) {
        match symbol.to_lowercase().as_str() {
            "epl" | "eng.1" => ("soccer", "eng.1"),
            "laliga" | "esp.1" => ("soccer", "esp.1"),
            "bundesliga" | "ger.1" => ("soccer", "ger.1"),
            "seriea" | "ita.1" => ("soccer", "ita.1"),
            "ligue1" | "fra.1" => ("soccer", "fra.1"),
            "mls" | "usa.1" => ("soccer", "usa.1"),
            "champions_league" | "uefa.champions" => ("soccer", "uefa.champions"),
            "nfl" => ("football", "nfl"),
            "nba" => ("basketball", "nba"),
            "mlb" => ("baseball", "mlb"),
            "nhl" => ("hockey", "nhl"),
            "atp" => ("tennis", "atp"),
            "wta" => ("tennis", "wta"),
            "ufc" | "mma" => ("mma", "ufc"),
            _ => ("soccer", "eng.1"),
        }
    }
}

impl HistoricalDataProvider for EspnProvider {
    fn id(&self) -> &str {
        "espn"
    }

    fn name(&self) -> &str {
        "ESPN Sports"
    }

    fn domain(&self) -> Domain {
        Domain::Sports
    }

    fn fetch_history(&self, query: &HistoryQuery) -> anyhow::Result<TimeSeries> {
        let (sport, league) = Self::resolve_league(&query.symbol);
        let url = format!(
            "https://site.api.espn.com/apis/site/v2/sports/{}/{}/scoreboard",
            sport, league
        );

        let resp = self.client.get(&url).send()?;
        let body: serde_json::Value = resp.json()?;
        let mut ts = TimeSeries::new("espn", &query.symbol);

        // ESPN scoreboard: {"events": [{"date": "...", "competitions": [...]}]}
        if let Some(events) = body.get("events").and_then(|e| e.as_array()) {
            for event in events {
                let date_str = event.get("date").and_then(|d| d.as_str()).unwrap_or("");

                let timestamp = chrono::DateTime::parse_from_rfc3339(date_str)
                    .map(|dt| dt.timestamp())
                    .unwrap_or(0);

                // Extract scores from first competition
                if let Some(comp) = event
                    .get("competitions")
                    .and_then(|c| c.as_array())
                    .and_then(|a| a.first())
                {
                    if let Some(competitors) = comp.get("competitors").and_then(|c| c.as_array()) {
                        let home_score = competitors
                            .iter()
                            .find(|c| {
                                c.get("homeAway")
                                    .and_then(|h| h.as_str())
                                    .map(|s| s == "home")
                                    .unwrap_or(false)
                            })
                            .and_then(|c| c.get("score"))
                            .and_then(|s| {
                                s.as_str()
                                    .and_then(|v| v.parse::<f64>().ok())
                                    .or_else(|| s.as_f64())
                            })
                            .unwrap_or(0.0);

                        let away_score = competitors
                            .iter()
                            .find(|c| {
                                c.get("homeAway")
                                    .and_then(|h| h.as_str())
                                    .map(|s| s == "away")
                                    .unwrap_or(false)
                            })
                            .and_then(|c| c.get("score"))
                            .and_then(|s| {
                                s.as_str()
                                    .and_then(|v| v.parse::<f64>().ok())
                                    .or_else(|| s.as_f64())
                            })
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
        let p = EspnProvider::new();
        assert_eq!(p.id(), "espn");
        assert_eq!(p.domain(), Domain::Sports);
    }

    #[test]
    fn league_resolution() {
        assert_eq!(EspnProvider::resolve_league("epl"), ("soccer", "eng.1"));
        assert_eq!(EspnProvider::resolve_league("nfl"), ("football", "nfl"));
        assert_eq!(EspnProvider::resolve_league("nba"), ("basketball", "nba"));
        assert_eq!(EspnProvider::resolve_league("ufc"), ("mma", "ufc"));
        assert_eq!(EspnProvider::resolve_league("atp"), ("tennis", "atp"));
    }
}
