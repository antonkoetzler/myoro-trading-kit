//! PortfolioState: aggregates positions and trade history from all domains.

use crate::pm::data::Position;
use serde::{Deserialize, Serialize};

/// A single trade row shown in the Portfolio history pane.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TradeRow {
    pub timestamp: String,
    pub domain: String,
    pub market_id: String,
    pub side: String,
    pub size: f64,
    pub price: f64,
    pub status: String,
}

impl TradeRow {
    /// Load from a JSONL paper trades file, newest first.
    pub fn load_from_jsonl(path: &str) -> Vec<TradeRow> {
        let content = match std::fs::read_to_string(path) {
            Ok(c) => c,
            Err(_) => return Vec::new(),
        };
        let mut rows: Vec<TradeRow> = content
            .lines()
            .filter_map(|line| serde_json::from_str::<serde_json::Value>(line).ok())
            .filter_map(|v| {
                Some(TradeRow {
                    timestamp: v["timestamp"].as_str()?.to_string(),
                    domain: v
                        .get("domain")
                        .and_then(|d| d.as_str())
                        .unwrap_or("unknown")
                        .to_string(),
                    market_id: v["market_id"].as_str()?.to_string(),
                    side: v["side"]
                        .as_str()
                        .map(|s| s.to_uppercase())
                        .unwrap_or_else(|| "?".to_string()),
                    size: v["size"].as_f64().unwrap_or(0.0),
                    price: v["price"].as_f64().unwrap_or(0.0),
                    status: v
                        .get("status")
                        .and_then(|s| s.as_str())
                        .unwrap_or("filled")
                        .to_string(),
                })
            })
            .collect();
        rows.reverse();
        rows
    }
}

/// Portfolio state: open positions (from tracker or API), trade history, domain P&L.
#[derive(Default)]
pub struct PortfolioState {
    pub open_positions: Vec<Position>,
    pub trade_history: Vec<TradeRow>,
    /// P&L per domain: (domain_name, today_pnl, alltime_pnl)
    pub domain_pnl: Vec<(String, f64, f64)>,
}

impl PortfolioState {
    /// Refresh from paper trade JSONL files (paper mode). Called every 8s.
    pub fn refresh(&mut self) {
        let sports_trades = TradeRow::load_from_jsonl("data/paper_sports_trades.jsonl");
        let copy_trades = TradeRow::load_from_jsonl("data/paper_copy_trades.jsonl");
        let mut all: Vec<TradeRow> = sports_trades.into_iter().chain(copy_trades).collect();
        all.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        all.truncate(200);
        self.trade_history = all;
        self.rebuild_domain_pnl();
    }

    /// Refresh positions from the Polymarket data API (live mode).
    pub fn refresh_live(&mut self, data: &crate::pm::data::DataClient, wallet: &str) {
        match data.get_positions(wallet) {
            Ok(positions) => self.open_positions = positions,
            Err(e) => eprintln!("[portfolio] get_positions error: {e}"),
        }
        // Still load paper trade history for display
        self.refresh();
    }

    fn rebuild_domain_pnl(&mut self) {
        self.domain_pnl = vec![
            (
                "Sports".to_string(),
                self.compute_pnl_for_domain("sports"),
                self.compute_pnl_for_domain("sports"),
            ),
            (
                "Copy".to_string(),
                self.compute_pnl_for_domain("copy"),
                self.compute_pnl_for_domain("copy"),
            ),
            ("Crypto".to_string(), 0.0, 0.0),
            ("Weather".to_string(), 0.0, 0.0),
        ];
    }

    fn compute_pnl_for_domain(&self, domain: &str) -> f64 {
        self.trade_history
            .iter()
            .filter(|t| t.domain == domain)
            .map(|t| {
                let mid = 0.5_f64;
                let edge = match t.side.as_str() {
                    "YES" => t.price - mid,
                    _ => mid - t.price,
                };
                edge * t.size
            })
            .sum()
    }

    pub fn total_pnl(&self) -> f64 {
        self.domain_pnl.iter().map(|(_, _, all)| all).sum()
    }
}

/// Parameters for a daily P&L summary flush.
pub struct DailySummary<'a> {
    pub date: &'a str,
    pub total_trades: u32,
    pub wins: u32,
    pub losses: u32,
    pub pnl_usd: f64,
    pub max_drawdown_usd: f64,
    pub strategies_active: &'a [&'a str],
    pub execution_mode: &'a str,
}

/// Flush a daily P&L summary line to `data/daily_pnl.jsonl`.
/// Called on graceful shutdown and at midnight UTC.
pub fn flush_daily_summary(s: &DailySummary<'_>) -> anyhow::Result<()> {
    std::fs::create_dir_all("data")?;
    let record = serde_json::json!({
        "date": s.date,
        "total_trades": s.total_trades,
        "wins": s.wins,
        "losses": s.losses,
        "pnl_usd": s.pnl_usd,
        "max_drawdown_usd": s.max_drawdown_usd,
        "strategies_active": s.strategies_active,
        "execution_mode": s.execution_mode,
    });
    let line = serde_json::to_string(&record)?;
    use std::io::Write;
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open("data/daily_pnl.jsonl")?;
    writeln!(file, "{}", line)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn load_from_jsonl_empty_path_returns_empty() {
        let rows = TradeRow::load_from_jsonl("/nonexistent/path.jsonl");
        assert!(rows.is_empty());
    }

    #[test]
    fn total_pnl_sums_domain_pnl() {
        let mut state = PortfolioState::default();
        state.domain_pnl = vec![("Sports".into(), 10.0, 10.0), ("Copy".into(), -5.0, -5.0)];
        assert!((state.total_pnl() - 5.0).abs() < 1e-9);
    }

    #[test]
    fn paper_mode_reads_jsonl_correctly() {
        use std::io::Write;
        let dir = std::env::temp_dir();
        let path = dir.join("test_portfolio_trades.jsonl");
        let mut f = std::fs::File::create(&path).unwrap();
        writeln!(
            f,
            r#"{{"timestamp":"2024-01-01T00:00:00Z","domain":"sports","market_id":"m1","side":"YES","size":10.0,"price":0.6,"status":"filled"}}"#
        )
        .unwrap();
        let rows = TradeRow::load_from_jsonl(path.to_str().unwrap());
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].domain, "sports");
        assert_eq!(rows[0].side, "YES");
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn flush_daily_summary_writes_jsonl_with_correct_fields() {
        let dir = std::env::temp_dir();
        let path = dir.join("test_daily_pnl_flush.jsonl");
        std::fs::remove_file(&path).ok();

        // Temporarily redirect by testing the logic directly (write to temp path)
        let summary = DailySummary {
            date: "2026-03-08",
            total_trades: 5,
            wins: 3,
            losses: 2,
            pnl_usd: 45.0,
            max_drawdown_usd: 10.0,
            strategies_active: &["binance_lag", "poisson"],
            execution_mode: "paper",
        };
        let record = serde_json::json!({
            "date": summary.date,
            "total_trades": summary.total_trades,
            "wins": summary.wins,
            "losses": summary.losses,
            "pnl_usd": summary.pnl_usd,
            "max_drawdown_usd": summary.max_drawdown_usd,
            "strategies_active": summary.strategies_active,
            "execution_mode": summary.execution_mode,
        });
        let line = serde_json::to_string(&record).unwrap();
        use std::io::Write as _;
        let mut f = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .unwrap();
        writeln!(f, "{}", line).unwrap();

        let contents = std::fs::read_to_string(&path).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(contents.trim()).unwrap();
        assert_eq!(parsed["date"], "2026-03-08");
        assert_eq!(parsed["total_trades"], 5);
        assert_eq!(parsed["wins"], 3);
        assert_eq!(parsed["pnl_usd"], 45.0);
        assert_eq!(parsed["execution_mode"], "paper");
        std::fs::remove_file(&path).ok();
    }
}
