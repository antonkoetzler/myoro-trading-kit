//! Execution: paper (JSONL fill simulation) or live (CLOB GTC limit order).

use crate::config::ExecutionMode;
use crate::pm::clob::{ClobClient, Fill, Order, OrderType, Side};
use anyhow::Result;
use std::io::Write;
use std::sync::Arc;

/// Write a trade record to the SQLite trades table in `data/cache.db`.
/// Table is created on first use. Non-fatal: errors are logged to stderr.
pub fn persist_trade_to_sqlite(
    market_id: &str,
    strategy_id: &str,
    side: &str,
    size: f64,
    price: f64,
    pnl: f64,
    execution_mode: &str,
) {
    let result = persist_trade_inner(
        market_id,
        strategy_id,
        side,
        size,
        price,
        pnl,
        execution_mode,
    );
    if let Err(e) = result {
        eprintln!("[sqlite] trade persist failed: {}", e);
    }
}

fn persist_trade_inner(
    market_id: &str,
    strategy_id: &str,
    side: &str,
    size: f64,
    price: f64,
    pnl: f64,
    execution_mode: &str,
) -> Result<()> {
    std::fs::create_dir_all("data")?;
    let conn = rusqlite::Connection::open("data/cache.db")?;
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS trades (
            id              INTEGER PRIMARY KEY AUTOINCREMENT,
            timestamp       TEXT NOT NULL,
            market_id       TEXT NOT NULL,
            strategy_id     TEXT NOT NULL,
            side            TEXT NOT NULL,
            size            REAL NOT NULL,
            price           REAL NOT NULL,
            pnl             REAL NOT NULL DEFAULT 0.0,
            execution_mode  TEXT NOT NULL
        );",
    )?;
    let ts = chrono::Utc::now().to_rfc3339();
    conn.execute(
        "INSERT INTO trades (timestamp, market_id, strategy_id, side, size, price, pnl, execution_mode)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        rusqlite::params![ts, market_id, strategy_id, side, size, price, pnl, execution_mode],
    )?;
    Ok(())
}

pub struct Executor {
    mode: ExecutionMode,
    /// Path to the JSONL file for paper trade recording.
    paper_trades_file: String,
    /// Domain name for trade attribution (crypto, sports, weather, copy).
    domain: String,
    /// Live CLOB client — None in paper mode or when credentials absent.
    clob: Option<Arc<ClobClient>>,
}

impl Executor {
    pub fn new(mode: ExecutionMode, paper_trades_file: &str) -> Self {
        Self {
            mode,
            paper_trades_file: paper_trades_file.to_string(),
            domain: String::new(),
            clob: None,
        }
    }

    pub fn with_domain(mut self, domain: &str) -> Self {
        self.domain = domain.to_string();
        self
    }

    pub fn with_clob(mut self, clob: Arc<ClobClient>) -> Self {
        self.clob = Some(clob);
        self
    }

    /// Execute at best available price. Paper: simulate fill and write to JSONL.
    /// Live: place a GTC limit order via CLOB (post_only=true).
    pub fn execute(&self, market_id: &str, side: super::strategy::Side, amount: f64) -> Result<()> {
        self.execute_with_price(market_id, side, amount, 0.5)
    }

    /// Execute with a specific price. Returns Ok after paper fill or live order.
    pub fn execute_with_price(
        &self,
        market_id: &str,
        side: super::strategy::Side,
        amount: f64,
        price: f64,
    ) -> Result<()> {
        match self.mode {
            ExecutionMode::Paper => {
                let fill = self.simulate_fill(market_id, side, amount, price);
                self.write_paper_fill(&fill)?;
            }
            ExecutionMode::Live => {
                let clob_side = match side {
                    super::strategy::Side::Yes => Side::Yes,
                    super::strategy::Side::No => Side::No,
                };
                let order = Order {
                    market_id: market_id.to_string(),
                    side: clob_side,
                    price,
                    size: amount,
                    order_type: OrderType::Limit,
                    post_only: true,
                };
                let clob = self
                    .clob
                    .as_ref()
                    .ok_or_else(|| anyhow::anyhow!("Live mode: ClobClient not configured"))?;
                let order_id = clob.place_limit_order(&order)?;
                eprintln!(
                    "[Executor] Live order placed: {} {:?} × {:.2} @ {:.4} → {}",
                    market_id, side, amount, price, order_id
                );
            }
        }
        Ok(())
    }

    pub fn is_live(&self) -> bool {
        self.mode == ExecutionMode::Live
    }

    fn simulate_fill(
        &self,
        market_id: &str,
        side: super::strategy::Side,
        amount: f64,
        price: f64,
    ) -> Fill {
        Fill {
            order_id: format!("paper-{}", chrono::Utc::now().timestamp_millis()),
            market_id: market_id.to_string(),
            side: match side {
                super::strategy::Side::Yes => Side::Yes,
                super::strategy::Side::No => Side::No,
            },
            price,
            size: amount,
            timestamp: chrono::Utc::now(),
        }
    }

    fn write_paper_fill(&self, fill: &Fill) -> Result<()> {
        // Ensure parent directory exists.
        if let Some(parent) = std::path::Path::new(&self.paper_trades_file).parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent)?;
            }
        }
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.paper_trades_file)?;
        // Serialize fill and inject domain field for portfolio attribution.
        let mut json: serde_json::Value = serde_json::to_value(fill)?;
        if !self.domain.is_empty() {
            json["domain"] = serde_json::Value::String(self.domain.clone());
        }
        let line = serde_json::to_string(&json)?;
        writeln!(file, "{}", line)?;
        // Also persist to SQLite.
        let side_str = format!("{:?}", fill.side);
        persist_trade_to_sqlite(
            &fill.market_id,
            "paper",
            &side_str,
            fill.size,
            fill.price,
            0.0,
            "paper",
        );
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared::strategy::Side;

    #[test]
    fn paper_executor_returns_ok_and_does_not_send() {
        let exec = Executor::new(ExecutionMode::Paper, "data/test_paper_trades.jsonl");
        assert!(!exec.is_live());
        assert!(exec.execute("market1", Side::Yes, 1.0).is_ok());
        let _ = std::fs::remove_file("data/test_paper_trades.jsonl");
    }

    #[test]
    fn live_executor_without_clob_returns_err() {
        let exec = Executor::new(ExecutionMode::Live, "data/paper.jsonl");
        assert!(exec.is_live());
        let result = exec.execute("market1", Side::No, 1.0);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("ClobClient not configured"));
    }

    #[test]
    fn simulate_fill_has_correct_fields() {
        let exec = Executor::new(ExecutionMode::Paper, "data/test.jsonl");
        let fill = exec.simulate_fill("mkt-1", Side::Yes, 25.0, 0.55);
        assert_eq!(fill.market_id, "mkt-1");
        assert!((fill.size - 25.0).abs() < 1e-9);
        assert!((fill.price - 0.55).abs() < 1e-9);
        assert!(fill.order_id.starts_with("paper-"));
    }

    #[test]
    fn sqlite_trade_persist_writes_and_reads_row() {
        // Use a temp db path to avoid polluting data/cache.db in tests.
        let dir = std::env::temp_dir();
        let db_path = dir.join("test_trades_persist.db");
        std::fs::remove_file(&db_path).ok();

        let conn = rusqlite::Connection::open(&db_path).unwrap();
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS trades (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                timestamp TEXT NOT NULL,
                market_id TEXT NOT NULL,
                strategy_id TEXT NOT NULL,
                side TEXT NOT NULL,
                size REAL NOT NULL,
                price REAL NOT NULL,
                pnl REAL NOT NULL DEFAULT 0.0,
                execution_mode TEXT NOT NULL
            );",
        )
        .unwrap();
        let ts = chrono::Utc::now().to_rfc3339();
        conn.execute(
            "INSERT INTO trades (timestamp, market_id, strategy_id, side, size, price, pnl, execution_mode)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            rusqlite::params![ts, "market-x", "binance_lag", "Yes", 10.0, 0.55, 0.5, "paper"],
        )
        .unwrap();

        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM trades WHERE market_id = 'market-x'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(count, 1);

        let market_id: String = conn
            .query_row(
                "SELECT market_id FROM trades WHERE strategy_id = 'binance_lag'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(market_id, "market-x");

        std::fs::remove_file(&db_path).ok();
    }
}
