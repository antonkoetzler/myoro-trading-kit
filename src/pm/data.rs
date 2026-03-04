//! Polymarket data API: positions and trade history.
//! Public endpoints — no authentication required.

use anyhow::Result;
use reqwest::blocking::Client;
use serde::Serialize;
use std::time::Duration;

/// An open position on a Polymarket market.
#[derive(Clone, Debug, serde::Deserialize, Serialize)]
pub struct Position {
    pub market_id: String,
    pub side: crate::pm::clob::Side,
    pub size: f64,
    pub avg_price: f64,
    pub current_price: f64,
    pub unrealized_pnl: f64,
    pub realized_pnl: f64,
}

impl Position {
    /// Total P&L = realized + unrealized.
    pub fn total_pnl(&self) -> f64 {
        self.realized_pnl + self.unrealized_pnl
    }
}

pub struct DataClient {
    host: String,
    http: Client,
}

impl DataClient {
    pub fn new(host: &str) -> Self {
        let http = Client::builder()
            .timeout(Duration::from_secs(15))
            .user_agent("myoro-polymarket-terminal/0.1")
            .build()
            .unwrap_or_default();
        Self {
            host: host.trim_end_matches('/').to_string(),
            http,
        }
    }

    /// Get open positions for a wallet address.
    pub fn get_positions(&self, wallet: &str) -> Result<Vec<Position>> {
        let url = format!(
            "{}/positions?user_address={}&sizeThreshold=.1",
            self.host, wallet
        );
        let body: Vec<serde_json::Value> = self.http.get(&url).send()?.json()?;
        Ok(body.into_iter().filter_map(parse_position).collect())
    }

    /// Get trade history for a wallet (newest first, up to `limit` entries).
    pub fn get_trades(&self, wallet: &str, limit: usize) -> Result<Vec<crate::pm::clob::Fill>> {
        let url = format!("{}/activity?user={}&limit={}", self.host, wallet, limit);
        let body: Vec<serde_json::Value> = self.http.get(&url).send()?.json()?;
        Ok(body.into_iter().filter_map(parse_fill).collect())
    }
}

fn parse_position(v: serde_json::Value) -> Option<Position> {
    let market_id = v
        .get("asset")
        .or_else(|| v.get("conditionId"))
        .and_then(|x| x.as_str())?
        .to_string();
    let side_str = v
        .get("outcome")
        .or_else(|| v.get("side"))
        .and_then(|x| x.as_str())
        .unwrap_or("YES");
    let side = if side_str.eq_ignore_ascii_case("YES") || side_str.eq_ignore_ascii_case("BUY") {
        crate::pm::clob::Side::Yes
    } else {
        crate::pm::clob::Side::No
    };
    Some(Position {
        market_id,
        side,
        size: v.get("size").and_then(|x| x.as_f64()).unwrap_or(0.0),
        avg_price: v.get("avgPrice").and_then(|x| x.as_f64()).unwrap_or(0.0),
        current_price: v
            .get("curPrice")
            .or_else(|| v.get("currentPrice"))
            .and_then(|x| x.as_f64())
            .unwrap_or(0.0),
        unrealized_pnl: v.get("cashPnl").and_then(|x| x.as_f64()).unwrap_or(0.0),
        realized_pnl: 0.0,
    })
}

fn parse_fill(v: serde_json::Value) -> Option<crate::pm::clob::Fill> {
    use crate::pm::clob::{Fill, Side};
    let order_id = v
        .get("id")
        .or_else(|| v.get("orderId"))
        .and_then(|x| x.as_str())?
        .to_string();
    let market_id = v
        .get("conditionId")
        .or_else(|| v.get("asset"))
        .and_then(|x| x.as_str())
        .unwrap_or("")
        .to_string();
    let side_str = v
        .get("outcome")
        .or_else(|| v.get("side"))
        .and_then(|x| x.as_str())
        .unwrap_or("YES");
    let side = if side_str.eq_ignore_ascii_case("YES") || side_str.eq_ignore_ascii_case("BUY") {
        Side::Yes
    } else {
        Side::No
    };
    let ts = v
        .get("timestamp")
        .and_then(|x| x.as_str())
        .unwrap_or("1970-01-01T00:00:00Z")
        .parse()
        .unwrap_or_else(|_| chrono::Utc::now());
    Some(Fill {
        order_id,
        market_id,
        side,
        price: v.get("price").and_then(|x| x.as_f64()).unwrap_or(0.0),
        size: v
            .get("amount")
            .or_else(|| v.get("size"))
            .and_then(|x| x.as_f64())
            .unwrap_or(0.0),
        timestamp: ts,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn position_total_pnl_sums_correctly() {
        let pos = Position {
            market_id: "m1".into(),
            side: crate::pm::clob::Side::Yes,
            size: 10.0,
            avg_price: 0.5,
            current_price: 0.6,
            unrealized_pnl: 1.0,
            realized_pnl: 0.5,
        };
        assert!((pos.total_pnl() - 1.5).abs() < 1e-9);
    }

    #[test]
    fn get_positions_parses_json_array() {
        let mut server = mockito::Server::new();
        let mock = server
            .mock("GET", mockito::Matcher::Regex(r"/positions".to_string()))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"[{"asset":"mkt1","outcome":"YES","size":100.0,"avgPrice":0.5,"curPrice":0.6,"cashPnl":10.0}]"#,
            )
            .create();
        let client = DataClient::new(&server.url());
        let positions = client.get_positions("0xwallet").unwrap();
        assert_eq!(positions.len(), 1);
        assert_eq!(positions[0].market_id, "mkt1");
        assert!((positions[0].size - 100.0).abs() < 1e-9);
        assert!((positions[0].avg_price - 0.5).abs() < 1e-9);
        assert!((positions[0].current_price - 0.6).abs() < 1e-9);
        assert!((positions[0].unrealized_pnl - 10.0).abs() < 1e-9);
        mock.assert();
    }

    #[test]
    fn get_positions_empty_wallet_returns_empty() {
        let mut server = mockito::Server::new();
        let mock = server
            .mock("GET", mockito::Matcher::Regex(r"/positions".to_string()))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body("[]")
            .create();
        let client = DataClient::new(&server.url());
        let positions = client.get_positions("0xempty").unwrap();
        assert!(positions.is_empty());
        mock.assert();
    }

    #[test]
    fn get_trades_parses_fills() {
        let mut server = mockito::Server::new();
        let mock = server
            .mock("GET", mockito::Matcher::Regex(r"/activity".to_string()))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"[{"id":"fill-001","conditionId":"mkt2","outcome":"NO","price":0.42,"amount":50.0,"timestamp":"2024-01-15T12:00:00Z"}]"#)
            .create();
        let client = DataClient::new(&server.url());
        let fills = client.get_trades("0xwallet", 10).unwrap();
        assert_eq!(fills.len(), 1);
        assert_eq!(fills[0].order_id, "fill-001");
        assert_eq!(fills[0].market_id, "mkt2");
        assert!((fills[0].price - 0.42).abs() < 1e-9);
        assert!((fills[0].size - 50.0).abs() < 1e-9);
        mock.assert();
    }

    #[test]
    fn get_positions_http_500_returns_err() {
        let mut server = mockito::Server::new();
        let _mock = server
            .mock("GET", mockito::Matcher::Regex(r"/positions".to_string()))
            .with_status(500)
            .with_body("internal error")
            .create();
        let client = DataClient::new(&server.url());
        assert!(client.get_positions("0xwallet").is_err());
    }
}
