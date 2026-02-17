//! Copy-trading: monitor trades from profiles listed in copy_traders.txt.
//! File: one 0x address per line (# comments ignored). TUI can add/remove; file changes are reloaded.

use serde::Deserialize;
use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::RwLock;

const DATA_API: &str = "https://data-api.polymarket.com";
const MAX_DISPLAY: usize = 24;
const DEFAULT_FILE: &str = "copy_traders.txt";

fn is_valid_address(s: &str) -> bool {
    let s = s.trim();
    s.starts_with("0x") && s.len() == 42 && s[2..].chars().all(|c| c.is_ascii_hexdigit())
}

fn normalize_address(s: &str) -> String {
    let s = s.trim();
    if s.starts_with("0x") {
        s.to_string()
    } else {
        format!("0x{}", s)
    }
}

/// One 0x address per line. Comments (#) and empty lines ignored. Reloads when file mtime changes.
pub struct TraderList {
    path: PathBuf,
    addresses: RwLock<Vec<String>>,
    last_mtime: RwLock<Option<std::time::SystemTime>>,
}

impl TraderList {
    pub fn path() -> PathBuf {
        std::env::var("COPY_TRADERS_FILE")
            .map(PathBuf::from)
            .unwrap_or_else(|_| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")).join(DEFAULT_FILE))
    }

    pub fn new() -> Self {
        let path = Self::path();
        let s = Self {
            path,
            addresses: RwLock::new(Vec::new()),
            last_mtime: RwLock::new(None),
        };
        s.load_from_file();
        s
    }

    pub fn load_from_file(&self) {
        let content = match std::fs::read_to_string(&self.path) {
            Ok(c) => c,
            Err(_) => return,
        };
        let addrs: Vec<String> = content
            .lines()
            .map(str::trim)
            .filter(|l| !l.is_empty() && !l.starts_with('#'))
            .filter_map(|l| {
                let n = normalize_address(l);
                if is_valid_address(&n) {
                    Some(n)
                } else {
                    None
                }
            })
            .collect();
        if let Ok(mut a) = self.addresses.write() {
            *a = addrs;
        }
        if let Ok(meta) = std::fs::metadata(&self.path) {
            if let Ok(mtime) = meta.modified() {
                if let Ok(mut last) = self.last_mtime.write() {
                    *last = Some(mtime);
                }
            }
        }
    }

    pub fn reload_if_changed(&self) {
        let meta = match std::fs::metadata(&self.path) {
            Ok(m) => m,
            Err(_) => return,
        };
        let mtime = match meta.modified() {
            Ok(t) => t,
            Err(_) => return,
        };
        let should_reload = self
            .last_mtime
            .read()
            .ok()
            .and_then(|last| (*last).map(|l| mtime > l))
            .unwrap_or(true);
        if should_reload {
            self.load_from_file();
        }
    }

    fn save_to_file(&self) {
        let addrs = match self.addresses.read() {
            Ok(a) => a.clone(),
            Err(_) => return,
        };
        let body = addrs.join("\n");
        let _ = std::fs::write(&self.path, body);
    }

    pub fn get_addresses(&self) -> Vec<String> {
        self.addresses.read().map(|a| a.clone()).unwrap_or_default()
    }

    pub fn add(&self, addr: String) -> bool {
        let n = normalize_address(&addr);
        if !is_valid_address(&n) {
            return false;
        }
        if let Ok(mut a) = self.addresses.write() {
            if a.contains(&n) {
                return true;
            }
            a.push(n);
            self.save_to_file();
            true
        } else {
            false
        }
    }

    pub fn remove_at(&self, index: usize) {
        if let Ok(mut a) = self.addresses.write() {
            if index < a.len() {
                a.remove(index);
                self.save_to_file();
            }
        }
    }

    pub fn len(&self) -> usize {
        self.addresses.read().map(|a| a.len()).unwrap_or(0)
    }
}

#[derive(Clone, Debug)]
pub struct TradeRow {
    pub user: String,
    pub side: String,
    pub size: f64,
    pub price: f64,
    pub title: String,
    pub outcome: String,
    pub ts: i64,
    pub tx: String,
}

#[derive(Debug, Deserialize)]
struct ApiTrade {
    #[serde(rename = "proxyWallet")]
    proxy_wallet: Option<String>,
    side: Option<String>,
    size: Option<f64>,
    price: Option<f64>,
    title: Option<String>,
    outcome: Option<String>,
    timestamp: Option<i64>,
    #[serde(rename = "transactionHash")]
    transaction_hash: Option<String>,
}

pub struct Monitor {
    list: std::sync::Arc<TraderList>,
    trades: RwLock<Vec<TradeRow>>,
    seen: RwLock<HashSet<String>>,
    log_sink: Option<std::sync::Arc<crate::live::LiveState>>,
    running: std::sync::Arc<AtomicBool>,
}

impl Monitor {
    pub fn poll_ms() -> u64 {
        std::env::var("COPY_POLL_MS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(250)
            .clamp(100, 30_000)
    }

    pub fn new(
        list: std::sync::Arc<TraderList>,
        log_sink: Option<std::sync::Arc<crate::live::LiveState>>,
        running: std::sync::Arc<AtomicBool>,
    ) -> Self {
        Self {
            list,
            trades: RwLock::new(Vec::new()),
            seen: RwLock::new(HashSet::new()),
            log_sink,
            running,
        }
    }

    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }

    pub fn set_running(&self, v: bool) {
        self.running.store(v, Ordering::SeqCst);
    }

    pub fn trader_list(&self) -> &std::sync::Arc<TraderList> {
        &self.list
    }

    pub fn poll_once(&self) {
        if !self.running.load(Ordering::SeqCst) {
            return;
        }
        self.list.reload_if_changed();
        let addresses = self.list.get_addresses();
        if addresses.is_empty() {
            return;
        }
        let client = match reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(8))
            .build()
        {
            Ok(c) => c,
            Err(_) => return,
        };
        let mut all: Vec<TradeRow> = Vec::new();
        for addr in &addresses {
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
                {
                    let mut seen = match self.seen.write() {
                        Ok(s) => s,
                        Err(_) => continue,
                    };
                    if seen.contains(&key) {
                        continue;
                    }
                    seen.insert(key);
                }
                all.push(TradeRow {
                    user: t.proxy_wallet.as_deref().unwrap_or("?").to_string(),
                    side: t.side.unwrap_or_else(|| "?".to_string()),
                    size: t.size.unwrap_or(0.0),
                    price: t.price.unwrap_or(0.0),
                    title: t.title.unwrap_or_else(|| "—".to_string()),
                    outcome: t.outcome.unwrap_or_else(|| "—".to_string()),
                    ts: t.timestamp.unwrap_or(0),
                    tx: tx.clone(),
                });
            }
        }
        if all.is_empty() {
            return;
        }
        all.sort_by(|a, b| b.ts.cmp(&a.ts));
        if let Some(ref live) = self.log_sink {
            for r in &all {
                live.push_log(format!(
                    "Copy trade: {} {} @ {} · {}",
                    r.side, r.size, r.price, r.title
                ));
            }
        }
        let mut trades = match self.trades.write() {
            Ok(t) => t,
            Err(_) => return,
        };
        for r in all {
            trades.insert(0, r);
        }
        trades.truncate(200);
    }

    pub fn copy_tab_display(
        &self,
        selected_index: Option<usize>,
        input_buf: &str,
    ) -> String {
        let addresses = self.list.get_addresses();
        let n = addresses.len();
        let ms = Self::poll_ms();
        let status = if self.is_running() { "Running" } else { "Stopped" };
        let mut out = format!(
            "Trading: {}  |  Profiles: {} (poll {}ms)  |  a=add d=remove ↑↓/jk select  |  s=start/stop  |  File: {}\n\n",
            status, n, ms, TraderList::path().display()
        );
        if !input_buf.is_empty() {
            out.push_str(&format!("Add address (Enter=save Esc=cancel): {}\n\n", input_buf));
        }
        for (i, addr) in addresses.iter().enumerate() {
            let mark = if Some(i) == selected_index { "► " } else { "  " };
            out.push_str(&format!("{}{}\n", mark, addr));
        }
        let trades = match self.trades.read() {
            Ok(t) => t,
            Err(_) => return out,
        };
        if !trades.is_empty() {
            out.push_str("\nRecent trades:\n");
            for r in trades.iter().take(MAX_DISPLAY) {
                let title_short = if r.title.len() > 40 {
                    format!("{}…", &r.title[..40])
                } else {
                    r.title.clone()
                };
                out.push_str(&format!(
                    "  {} {} {} @ {} · {}\n",
                    r.side,
                    r.size,
                    r.price,
                    r.outcome,
                    title_short
                ));
            }
        }
        out
    }
}
