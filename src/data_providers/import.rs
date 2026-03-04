//! CSV/JSON file import data provider.
//! Auto-detects columns by headers (timestamp, price, open, close, score, etc.).
use super::{DataPoint, HistoricalDataProvider, HistoryQuery, TimeSeries};
use crate::strategy_engine::Domain;
use std::path::Path;

pub struct ImportProvider;

impl Default for ImportProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl ImportProvider {
    pub fn new() -> Self {
        Self
    }

    /// Load a CSV file as TimeSeries. Auto-detects timestamp and value columns.
    pub fn load_csv(path: &str) -> anyhow::Result<TimeSeries> {
        let content = std::fs::read_to_string(path)?;
        let mut lines = content.lines();
        let header = lines.next().unwrap_or("");
        let columns: Vec<&str> = header.split(',').map(|s| s.trim()).collect();

        // Find timestamp column
        let ts_idx = columns
            .iter()
            .position(|&c| {
                matches!(
                    c.to_lowercase().as_str(),
                    "timestamp" | "time" | "date" | "ts" | "datetime"
                )
            })
            .unwrap_or(0);

        // Value columns are everything else that's numeric
        let mut ts = TimeSeries::new("import", path);

        for line in lines {
            let fields: Vec<&str> = line.split(',').map(|s| s.trim()).collect();
            if fields.len() < 2 {
                continue;
            }

            let timestamp = parse_timestamp(fields.get(ts_idx).unwrap_or(&""));
            let mut values = Vec::new();

            for (i, &col) in columns.iter().enumerate() {
                if i == ts_idx {
                    continue;
                }
                if let Some(val) = fields.get(i).and_then(|f| f.parse::<f64>().ok()) {
                    values.push((col.to_string(), val));
                }
            }

            if !values.is_empty() {
                ts.points.push(DataPoint { timestamp, values });
            }
        }

        Ok(ts)
    }

    /// Load a JSONL file as TimeSeries.
    pub fn load_jsonl(path: &str) -> anyhow::Result<TimeSeries> {
        let content = std::fs::read_to_string(path)?;
        let mut ts = TimeSeries::new("import", path);

        for line in content.lines() {
            if line.trim().is_empty() {
                continue;
            }
            if let Ok(obj) = serde_json::from_str::<serde_json::Value>(line) {
                let timestamp = obj
                    .get("timestamp")
                    .or_else(|| obj.get("ts"))
                    .or_else(|| obj.get("time"))
                    .and_then(|v| v.as_i64())
                    .unwrap_or(0);

                let mut values = Vec::new();
                if let Some(map) = obj.as_object() {
                    for (k, v) in map {
                        if matches!(k.as_str(), "timestamp" | "ts" | "time") {
                            continue;
                        }
                        if let Some(f) = v.as_f64() {
                            values.push((k.clone(), f));
                        }
                    }
                }

                if !values.is_empty() {
                    ts.points.push(DataPoint { timestamp, values });
                }
            }
        }

        Ok(ts)
    }
}

impl HistoricalDataProvider for ImportProvider {
    fn id(&self) -> &str {
        "import"
    }

    fn name(&self) -> &str {
        "Import CSV/JSON"
    }

    fn domain(&self) -> Domain {
        Domain::All
    }

    fn fetch_history(&self, query: &HistoryQuery) -> anyhow::Result<TimeSeries> {
        // Try multiple paths
        let candidates = [
            format!("data/import/{}.csv", query.symbol),
            format!("data/import/{}.jsonl", query.symbol),
            format!("data/import/{}.json", query.symbol),
            query.symbol.clone(),
        ];

        for path in &candidates {
            if !Path::new(path).exists() {
                continue;
            }
            let ext = Path::new(path)
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("");
            return match ext {
                "csv" => Self::load_csv(path),
                "jsonl" | "json" => Self::load_jsonl(path),
                _ => Self::load_csv(path),
            };
        }

        anyhow::bail!("No import file found for symbol: {}", query.symbol)
    }
}

/// Parse various timestamp formats.
fn parse_timestamp(s: &str) -> i64 {
    // Try unix timestamp
    if let Ok(ts) = s.parse::<i64>() {
        return ts;
    }
    // Try ISO 8601
    if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(s) {
        return dt.timestamp();
    }
    // Try YYYY-MM-DD
    if let Ok(d) = chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d") {
        return d
            .and_hms_opt(0, 0, 0)
            .unwrap_or_default()
            .and_utc()
            .timestamp();
    }
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_timestamps() {
        assert!(parse_timestamp("1700000000") > 0);
        assert!(parse_timestamp("2024-06-15") > 0);
        assert_eq!(parse_timestamp("invalid"), 0);
    }

    #[test]
    fn provider_metadata() {
        let p = ImportProvider::new();
        assert_eq!(p.id(), "import");
        assert_eq!(p.domain(), Domain::All);
    }

    #[test]
    fn missing_file_returns_error() {
        let p = ImportProvider::new();
        let q = HistoryQuery::last_days("nonexistent_symbol", 30);
        assert!(p.fetch_history(&q).is_err());
    }
}
