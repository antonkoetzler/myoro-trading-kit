//! HTTP fetching of copy trades + JSONL append for paper trades.

use std::io::Write;
use std::path::Path;

use crate::copy_trading::types::{ApiTrade, PaperTradeRecord, TradeRow};

const DATA_API: &str = "https://data-api.polymarket.com";

/// Fetch recent trades for a list of trader addresses. Returns new trades only if seen is tracked externally.
pub(super) fn fetch_recent_trades(
    addresses: &[String],
    seen: &mut std::collections::HashSet<String>,
) -> Vec<TradeRow> {
    let client = match reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(8))
        .build()
    {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };
    let mut all: Vec<TradeRow> = Vec::new();
    for addr in addresses {
        let url = format!("{}/trades?user={}&limit=30&takerOnly=false", DATA_API, addr);
        let resp = match client.get(&url).send() {
            Ok(r) => r,
            Err(_) => continue,
        };
        let list: Vec<ApiTrade> = match resp.json() {
            Ok(l) => l,
            Err(_) => continue,
        };
        for t in list {
            let tx = t.transaction_hash.unwrap_or_default();
            if tx.is_empty() {
                continue;
            }
            let key = format!("{}:{}", tx, t.timestamp.unwrap_or(0));
            if seen.contains(&key) {
                continue;
            }
            seen.insert(key);
            all.push(TradeRow {
                user: t.proxy_wallet.as_deref().unwrap_or("?").to_string(),
                side: t.side.unwrap_or_else(|| "?".to_string()),
                size: t.size.unwrap_or(0.0),
                price: t.price.unwrap_or(0.0),
                title: t.title.unwrap_or_else(|| "—".to_string()),
                outcome: t.outcome.unwrap_or_else(|| "—".to_string()),
                ts: t.timestamp.unwrap_or(0),
                tx: tx.clone(),
                condition_id: t.condition_id.clone(),
                asset_id: t.asset.clone(),
            });
        }
    }
    all
}

pub(super) fn append_paper_trade_jsonl(
    path: &str,
    trade: &TradeRow,
    size: f64,
) -> anyhow::Result<()> {
    let path = Path::new(path);
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)?;
        }
    }
    let record = PaperTradeRecord {
        timestamp: chrono::Utc::now().to_rfc3339(),
        source_timestamp: trade.ts,
        condition_id: trade.condition_id.as_deref().unwrap_or_default(),
        asset_id: trade.asset_id.as_deref(),
        side: &trade.side,
        size,
        price: trade.price,
        title: &trade.title,
        outcome: &trade.outcome,
        source_trader_address: &trade.user,
        source_transaction_hash: &trade.tx,
    };
    let json = serde_json::to_string(&record)?;
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?;
    writeln!(file, "{}", json)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn append_paper_trade_writes_jsonl() {
        let ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("time")
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("paper_copy_test_{}", ts));
        std::fs::create_dir_all(&dir).expect("mkdir");
        let file = dir.join("paper_copy_trades.jsonl");
        let trade = TradeRow {
            user: "0xabc".to_string(),
            side: "BUY".to_string(),
            size: 2.0,
            price: 0.6,
            title: "Market".to_string(),
            outcome: "YES".to_string(),
            ts: 10,
            tx: "0xtx".to_string(),
            condition_id: Some("0xcond".to_string()),
            asset_id: Some("123".to_string()),
        };
        append_paper_trade_jsonl(file.to_str().expect("path"), &trade, 1.25).expect("append");
        let body = std::fs::read_to_string(&file).expect("read");
        assert!(body.contains("\"condition_id\":\"0xcond\""));
        assert!(body.contains("\"size\":1.25"));
        let _ = std::fs::remove_dir_all(dir);
    }
}
