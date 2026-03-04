//! CLOB order placement, cancellation, and fill retrieval.
//! Uses reqwest::blocking for synchronous execution consistent with the rest of the codebase.
//! NOTE: Polymarket CLOB orders require EIP-712 signed typed-data. The auth header structure
//! is in place; the signing step requires private-key credentials to be wired via the SDK.

use anyhow::{anyhow, Result};
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Direction of a trade.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum Side {
    Yes,
    No,
}

/// Order type sent to the CLOB.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum OrderType {
    Limit,
    Market,
}

/// An order to be submitted to the CLOB.
#[derive(Clone, Debug, Serialize)]
pub struct Order {
    pub market_id: String,
    pub side: Side,
    /// Limit price in USDC (0.0–1.0 on binary markets).
    pub price: f64,
    /// Size in USDC.
    pub size: f64,
    pub order_type: OrderType,
    /// Post-only: only place if the order will be a maker (GTC limit, no taker fee).
    pub post_only: bool,
}

/// A confirmed fill from the CLOB.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Fill {
    pub order_id: String,
    pub market_id: String,
    pub side: Side,
    pub price: f64,
    pub size: f64,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// Credentials for CLOB L1 auth.
pub struct ClobAuth {
    pub api_key: String,
    pub api_secret: String,
    pub api_passphrase: String,
    /// Polymarket proxy (Safe) address — the "owner" of orders.
    pub funder_address: String,
}

pub struct ClobClient {
    host: String,
    auth: Option<ClobAuth>,
    http: Client,
}

impl ClobClient {
    pub fn new(host: &str) -> Self {
        let http = Client::builder()
            .timeout(Duration::from_secs(15))
            .user_agent("myoro-polymarket-terminal/0.1")
            .build()
            .unwrap_or_default();
        Self {
            host: host.trim_end_matches('/').to_string(),
            auth: None,
            http,
        }
    }

    pub fn with_auth(mut self, auth: ClobAuth) -> Self {
        self.auth = Some(auth);
        self
    }

    /// Place a limit order (GTC). Returns the order_id on success.
    /// Requires CLOB credentials; returns Err if not configured.
    pub fn place_limit_order(&self, order: &Order) -> Result<String> {
        let auth = self.auth.as_ref().ok_or_else(|| {
            anyhow!("CLOB credentials not configured — set API_KEY/PASSPHRASE or PRIVATE_KEY")
        })?;
        let body = build_order_body(order, auth);
        let resp: serde_json::Value = self
            .http
            .post(format!("{}/order", self.host))
            .header("POLY-ADDRESS", &auth.funder_address)
            .header("POLY-API-KEY", &auth.api_key)
            .header("POLY-PASSPHRASE", &auth.api_passphrase)
            .header("POLY-TIMESTAMP", chrono::Utc::now().timestamp().to_string())
            .json(&body)
            .send()?
            .error_for_status()?
            .json()?;
        extract_order_id(&resp)
    }

    /// Place a market order. Returns the order_id.
    pub fn place_market_order(&self, order: &Order) -> Result<String> {
        let auth = self.auth.as_ref().ok_or_else(|| {
            anyhow!("CLOB credentials not configured — set API_KEY/PASSPHRASE or PRIVATE_KEY")
        })?;
        let mut body = build_order_body(order, auth);
        body["orderType"] = serde_json::json!("FOK");
        let resp: serde_json::Value = self
            .http
            .post(format!("{}/order", self.host))
            .header("POLY-ADDRESS", &auth.funder_address)
            .header("POLY-API-KEY", &auth.api_key)
            .header("POLY-PASSPHRASE", &auth.api_passphrase)
            .header("POLY-TIMESTAMP", chrono::Utc::now().timestamp().to_string())
            .json(&body)
            .send()?
            .error_for_status()?
            .json()?;
        extract_order_id(&resp)
    }

    /// Cancel an open order.
    pub fn cancel_order(&self, order_id: &str) -> Result<()> {
        let auth = self.auth.as_ref().ok_or_else(|| {
            anyhow!("CLOB credentials not configured — set API_KEY/PASSPHRASE or PRIVATE_KEY")
        })?;
        self.http
            .delete(format!("{}/order/{}", self.host, order_id))
            .header("POLY-ADDRESS", &auth.funder_address)
            .header("POLY-API-KEY", &auth.api_key)
            .header("POLY-PASSPHRASE", &auth.api_passphrase)
            .header("POLY-TIMESTAMP", chrono::Utc::now().timestamp().to_string())
            .send()?
            .error_for_status()?;
        Ok(())
    }

    /// Get fill status for an order_id.
    pub fn get_fill(&self, order_id: &str) -> Result<Option<Fill>> {
        let resp = self
            .http
            .get(format!("{}/order/{}", self.host, order_id))
            .send()?;
        if resp.status() == reqwest::StatusCode::NOT_FOUND {
            return Ok(None);
        }
        let v: serde_json::Value = resp.error_for_status()?.json()?;
        let filled = v.get("status").and_then(|s| s.as_str()).unwrap_or("") == "MATCHED";
        if !filled {
            return Ok(None);
        }
        let side_str = v.get("side").and_then(|s| s.as_str()).unwrap_or("YES");
        let side = if side_str.eq_ignore_ascii_case("BUY") || side_str.eq_ignore_ascii_case("YES") {
            Side::Yes
        } else {
            Side::No
        };
        Ok(Some(Fill {
            order_id: order_id.to_string(),
            market_id: v
                .get("asset")
                .and_then(|x| x.as_str())
                .unwrap_or("")
                .to_string(),
            side,
            price: v.get("price").and_then(|x| x.as_f64()).unwrap_or(0.0),
            size: v.get("size").and_then(|x| x.as_f64()).unwrap_or(0.0),
            timestamp: chrono::Utc::now(),
        }))
    }
}

fn build_order_body(order: &Order, auth: &ClobAuth) -> serde_json::Value {
    let side_str = match order.side {
        Side::Yes => "BUY",
        Side::No => "SELL",
    };
    serde_json::json!({
        "order": {
            "tokenId": order.market_id,
            "price": format!("{:.6}", order.price),
            "side": side_str,
            "size": format!("{:.6}", order.size),
            "expiration": "0",
            "nonce": "0",
            "feeRateBps": "0",
            "signatureType": 1,
            // Signature placeholder — real signing requires secp256k1 private key
            "signature": "0x"
        },
        "owner": auth.funder_address,
        "orderType": "GTC"
    })
}

fn extract_order_id(resp: &serde_json::Value) -> Result<String> {
    resp.get("orderID")
        .or_else(|| resp.get("orderId"))
        .or_else(|| resp.get("order_id"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| anyhow!("CLOB: no orderID in response: {}", resp))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn auth() -> ClobAuth {
        ClobAuth {
            api_key: "test-key".into(),
            api_secret: "test-secret".into(),
            api_passphrase: "test-pass".into(),
            funder_address: "0x0000000000000000000000000000000000000000".into(),
        }
    }

    fn limit_order() -> Order {
        Order {
            market_id: "market-001".into(),
            side: Side::Yes,
            price: 0.55,
            size: 10.0,
            order_type: OrderType::Limit,
            post_only: true,
        }
    }

    #[test]
    fn place_limit_parses_order_id() {
        let mut server = mockito::Server::new();
        let mock = server
            .mock("POST", "/order")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"orderID":"0xabc123def456"}"#)
            .create();
        let client = ClobClient::new(&server.url()).with_auth(auth());
        let id = client.place_limit_order(&limit_order()).unwrap();
        assert_eq!(id, "0xabc123def456");
        mock.assert();
    }

    #[test]
    fn cancel_order_succeeds() {
        let mut server = mockito::Server::new();
        let mock = server
            .mock("DELETE", "/order/0xabc")
            .with_status(200)
            .create();
        let client = ClobClient::new(&server.url()).with_auth(auth());
        assert!(client.cancel_order("0xabc").is_ok());
        mock.assert();
    }

    #[test]
    fn place_order_auth_failure_propagates() {
        let mut server = mockito::Server::new();
        let _mock = server
            .mock("POST", "/order")
            .with_status(401)
            .with_body("Unauthorized")
            .create();
        let client = ClobClient::new(&server.url()).with_auth(auth());
        assert!(client.place_limit_order(&limit_order()).is_err());
    }

    #[test]
    fn no_auth_returns_err_without_request() {
        let client = ClobClient::new("https://clob.polymarket.com");
        let result = client.place_limit_order(&limit_order());
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("credentials not configured"));
    }
}
