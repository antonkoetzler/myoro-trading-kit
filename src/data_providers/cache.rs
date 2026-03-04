//! SQLite local cache with WAL mode and tiered invalidation.
//!
//! Cache tiers:
//! - Hot (today): re-fetch every 60s during active events
//! - Warm (last 7 days): re-fetch once per hour
//! - Cold (>7 days): immutable, never re-fetch
use rusqlite::{params, Connection};
use std::path::Path;

const DEFAULT_DB_PATH: &str = "data/cache.db";

/// SQLite-backed data cache.
pub struct DataCache {
    conn: Connection,
}

impl DataCache {
    /// Open or create the cache database.
    pub fn open(path: &str) -> anyhow::Result<Self> {
        // Ensure parent directory exists
        if let Some(parent) = Path::new(path).parent() {
            std::fs::create_dir_all(parent)?;
        }

        let conn = Connection::open(path)?;
        conn.execute_batch(
            "PRAGMA journal_mode = WAL;
             PRAGMA synchronous = NORMAL;
             PRAGMA cache_size = -64000;
             PRAGMA temp_store = MEMORY;",
        )?;

        let cache = Self { conn };
        cache.create_tables()?;
        Ok(cache)
    }

    /// Open with default path.
    pub fn open_default() -> anyhow::Result<Self> {
        Self::open(DEFAULT_DB_PATH)
    }

    fn create_tables(&self) -> anyhow::Result<()> {
        self.conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS time_series (
                source TEXT NOT NULL,
                symbol TEXT NOT NULL,
                timestamp INTEGER NOT NULL,
                key TEXT NOT NULL,
                value REAL NOT NULL,
                fetched_at INTEGER NOT NULL,
                PRIMARY KEY (source, symbol, timestamp, key)
            );
            CREATE INDEX IF NOT EXISTS idx_ts_lookup
                ON time_series(source, symbol, timestamp);
            CREATE TABLE IF NOT EXISTS cache_meta (
                source TEXT NOT NULL,
                symbol TEXT NOT NULL,
                last_fetch INTEGER NOT NULL,
                point_count INTEGER NOT NULL DEFAULT 0,
                PRIMARY KEY (source, symbol)
            );",
        )?;
        Ok(())
    }

    /// Store a TimeSeries in the cache.
    pub fn store(&self, ts: &super::TimeSeries) -> anyhow::Result<()> {
        let tx = self.conn.unchecked_transaction()?;
        let now = chrono::Utc::now().timestamp();

        {
            let mut stmt = tx.prepare_cached(
                "INSERT OR REPLACE INTO time_series (source, symbol, timestamp, key, value, fetched_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            )?;

            for point in &ts.points {
                for (key, value) in &point.values {
                    stmt.execute(params![
                        ts.source,
                        ts.symbol,
                        point.timestamp,
                        key,
                        value,
                        now,
                    ])?;
                }
            }
        }

        tx.execute(
            "INSERT OR REPLACE INTO cache_meta (source, symbol, last_fetch, point_count)
             VALUES (?1, ?2, ?3, ?4)",
            params![ts.source, ts.symbol, now, ts.points.len()],
        )?;

        tx.commit()?;
        Ok(())
    }

    /// Load a TimeSeries from the cache.
    pub fn load(
        &self,
        source: &str,
        symbol: &str,
        start_ts: i64,
        end_ts: i64,
    ) -> anyhow::Result<super::TimeSeries> {
        let mut stmt = self.conn.prepare_cached(
            "SELECT timestamp, key, value FROM time_series
             WHERE source = ?1 AND symbol = ?2 AND timestamp >= ?3 AND timestamp <= ?4
             ORDER BY timestamp, key",
        )?;

        let mut ts = super::TimeSeries::new(source, symbol);
        let mut current_ts: Option<i64> = None;
        let mut current_values: Vec<(String, f64)> = Vec::new();

        let rows = stmt.query_map(params![source, symbol, start_ts, end_ts], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, f64>(2)?,
            ))
        })?;

        for row in rows {
            let (t, key, value) = row?;
            if current_ts != Some(t) {
                if let Some(prev_ts) = current_ts {
                    ts.points.push(super::DataPoint {
                        timestamp: prev_ts,
                        values: std::mem::take(&mut current_values),
                    });
                }
                current_ts = Some(t);
            }
            current_values.push((key, value));
        }

        // Flush last point
        if let Some(prev_ts) = current_ts {
            ts.points.push(super::DataPoint {
                timestamp: prev_ts,
                values: current_values,
            });
        }

        Ok(ts)
    }

    /// Check if we should re-fetch based on cache tier.
    pub fn should_refetch(&self, source: &str, symbol: &str) -> bool {
        let now = chrono::Utc::now().timestamp();
        let last_fetch = self
            .conn
            .query_row(
                "SELECT last_fetch FROM cache_meta WHERE source = ?1 AND symbol = ?2",
                params![source, symbol],
                |row| row.get::<_, i64>(0),
            )
            .unwrap_or(0);

        if last_fetch == 0 {
            return true; // Never fetched
        }

        let age = now - last_fetch;
        let data_age = now - last_fetch;

        if data_age < 86400 {
            age > 60 // Hot: re-fetch every 60s
        } else if data_age < 7 * 86400 {
            age > 3600 // Warm: re-fetch every hour
        } else {
            false // Cold: never re-fetch
        }
    }

    /// Delete all cached data for a source+symbol.
    pub fn invalidate(&self, source: &str, symbol: &str) -> anyhow::Result<()> {
        self.conn.execute(
            "DELETE FROM time_series WHERE source = ?1 AND symbol = ?2",
            params![source, symbol],
        )?;
        self.conn.execute(
            "DELETE FROM cache_meta WHERE source = ?1 AND symbol = ?2",
            params![source, symbol],
        )?;
        Ok(())
    }

    /// Get cache statistics.
    pub fn stats(&self) -> anyhow::Result<Vec<(String, String, i64, i64)>> {
        let mut stmt = self.conn.prepare(
            "SELECT source, symbol, last_fetch, point_count FROM cache_meta ORDER BY last_fetch DESC",
        )?;
        let rows = stmt
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, i64>(3)?,
                ))
            })?
            .filter_map(|r| r.ok())
            .collect();
        Ok(rows)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data_providers::{DataPoint, TimeSeries};

    #[test]
    fn roundtrip_store_load() {
        let cache = DataCache::open(":memory:").unwrap();
        let mut ts = TimeSeries::new("test", "BTC");
        ts.points.push(DataPoint {
            timestamp: 1000,
            values: vec![("close".into(), 42.0), ("volume".into(), 100.0)],
        });
        ts.points.push(DataPoint {
            timestamp: 2000,
            values: vec![("close".into(), 43.0), ("volume".into(), 110.0)],
        });

        cache.store(&ts).unwrap();
        let loaded = cache.load("test", "BTC", 0, 3000).unwrap();
        assert_eq!(loaded.len(), 2);
        assert_eq!(loaded.column("close"), vec![42.0, 43.0]);
    }

    #[test]
    fn cache_invalidation() {
        let cache = DataCache::open(":memory:").unwrap();
        let mut ts = TimeSeries::new("test", "ETH");
        ts.points.push(DataPoint {
            timestamp: 1000,
            values: vec![("price".into(), 3000.0)],
        });
        cache.store(&ts).unwrap();
        cache.invalidate("test", "ETH").unwrap();
        let loaded = cache.load("test", "ETH", 0, 9999).unwrap();
        assert!(loaded.is_empty());
    }

    #[test]
    fn should_refetch_unknown_source() {
        let cache = DataCache::open(":memory:").unwrap();
        assert!(cache.should_refetch("unknown", "XYZ"));
    }

    #[test]
    fn stats_empty() {
        let cache = DataCache::open(":memory:").unwrap();
        let stats = cache.stats().unwrap();
        assert!(stats.is_empty());
    }
}
