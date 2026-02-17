//! Live data: Crypto (Gamma + Binance), Sports, Weather. Fetched in background; TUI reads.

use std::sync::RwLock;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LogLevel {
    Info,
    Success,
    Warning,
    Error,
}

const GAMMA_EVENTS: &str = "https://gamma-api.polymarket.com/events?closed=false&limit=15";
const BINANCE_TICKER: &str = "https://api.binance.com/api/v3/ticker/price?symbol=BTCUSDT";

#[derive(Default)]
pub struct CryptoState {
    pub btc_usdt: String,
    pub events: Vec<String>,
}

#[derive(Default)]
pub struct SportsState {
    pub fixtures: Vec<String>,
}

#[derive(Default)]
pub struct WeatherState {
    pub forecast: Vec<String>,
}

const MAX_LOGS: usize = 80;

fn truncate_log(g: &mut Vec<(LogLevel, String)>) {
    let drop = g.len().saturating_sub(MAX_LOGS);
    if drop > 0 {
        g.drain(0..drop);
    }
}

/// Global stats shown on every tab.
pub struct GlobalStats {
    pub bankroll: Option<f64>,
    pub pnl: f64,
    pub open_trades: u32,
    pub closed_trades: u32,
}

impl Default for GlobalStats {
    fn default() -> Self {
        Self {
            bankroll: None,
            pnl: 0.0,
            open_trades: 0,
            closed_trades: 0,
        }
    }
}

pub struct LiveState {
    pub crypto: RwLock<CryptoState>,
    pub sports: RwLock<SportsState>,
    pub weather: RwLock<WeatherState>,
    pub crypto_logs: RwLock<Vec<(LogLevel, String)>>,
    pub sports_logs: RwLock<Vec<(LogLevel, String)>>,
    pub weather_logs: RwLock<Vec<(LogLevel, String)>>,
    pub copy_logs: RwLock<Vec<(LogLevel, String)>>,
    pub discover_logs: RwLock<Vec<(LogLevel, String)>>,
    pub global_stats: RwLock<GlobalStats>,
}

impl Default for LiveState {
    fn default() -> Self {
        Self {
            crypto: RwLock::new(CryptoState::default()),
            sports: RwLock::new(SportsState::default()),
            weather: RwLock::new(WeatherState::default()),
            crypto_logs: RwLock::new(Vec::new()),
            sports_logs: RwLock::new(Vec::new()),
            weather_logs: RwLock::new(Vec::new()),
            copy_logs: RwLock::new(Vec::new()),
            discover_logs: RwLock::new(Vec::new()),
            global_stats: RwLock::new(GlobalStats::default()),
        }
    }
}

impl LiveState {
    pub fn push_log(&self, s: String) {
        self.push_copy_log(LogLevel::Info, s);
    }

    pub fn push_crypto_log(&self, level: LogLevel, s: String) {
        if let Ok(mut g) = self.crypto_logs.write() {
            g.push((level, s));
            truncate_log(&mut g);
        }
    }
    pub fn push_sports_log(&self, level: LogLevel, s: String) {
        if let Ok(mut g) = self.sports_logs.write() {
            g.push((level, s));
            truncate_log(&mut g);
        }
    }
    pub fn push_weather_log(&self, level: LogLevel, s: String) {
        if let Ok(mut g) = self.weather_logs.write() {
            g.push((level, s));
            truncate_log(&mut g);
        }
    }
    pub fn push_copy_log(&self, level: LogLevel, s: String) {
        if let Ok(mut g) = self.copy_logs.write() {
            g.push((level, s));
            truncate_log(&mut g);
        }
    }

    pub fn get_crypto_logs(&self) -> Vec<(LogLevel, String)> {
        self.crypto_logs.read().map(|g| g.clone()).unwrap_or_default()
    }
    pub fn get_sports_logs(&self) -> Vec<(LogLevel, String)> {
        self.sports_logs.read().map(|g| g.clone()).unwrap_or_default()
    }
    pub fn get_weather_logs(&self) -> Vec<(LogLevel, String)> {
        self.weather_logs.read().map(|g| g.clone()).unwrap_or_default()
    }
    pub fn get_copy_logs(&self) -> Vec<(LogLevel, String)> {
        self.copy_logs.read().map(|g| g.clone()).unwrap_or_default()
    }
    pub fn get_discover_logs(&self) -> Vec<(LogLevel, String)> {
        self.discover_logs.read().map(|g| g.clone()).unwrap_or_default()
    }

    pub fn last_log_is_error(&self, tab: u8) -> bool {
        let logs = match tab {
            0 => self.get_crypto_logs(),
            1 => self.get_sports_logs(),
            2 => self.get_weather_logs(),
            _ => return false,
        };
        logs.last().map(|(l, _)| *l == LogLevel::Error).unwrap_or(false)
    }

    pub fn set_bankroll(&self, v: Option<f64>) {
        if let Ok(mut s) = self.global_stats.write() {
            s.bankroll = v;
        }
    }
}

impl LiveState {
    pub fn fetch_all(&self) {
        if let Ok(client) = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .build()
        {
            self.push_crypto_log(LogLevel::Info, "[Crypto] Fetching BTC/USDT (Binance)…".to_string());
            if let Ok(resp) = client.get(BINANCE_TICKER).send() {
                if let Ok(json) = resp.json::<serde_json::Value>() {
                    let price = json.get("price").and_then(|p| p.as_str()).unwrap_or("—");
                    if let Ok(mut c) = self.crypto.write() {
                        c.btc_usdt = format!("BTC/USDT {}", price);
                    }
                    self.push_crypto_log(LogLevel::Success, format!("[Crypto] BTC/USDT {}", price));
                } else {
                    self.push_crypto_log(LogLevel::Warning, "[Crypto] Binance ticker parse failed".to_string());
                }
            } else {
                self.push_crypto_log(LogLevel::Error, "[Crypto] Binance request failed".to_string());
            }
            self.push_crypto_log(LogLevel::Info, "[Crypto] Fetching Gamma events…".to_string());
            if let Ok(resp) = client.get(GAMMA_EVENTS).send() {
                if let Ok(arr) = resp.json::<Vec<serde_json::Value>>() {
                    let lines: Vec<String> = arr
                        .iter()
                        .take(10)
                        .filter_map(|e| {
                            let title = e.get("title").and_then(|t| t.as_str())?;
                            let slug = e.get("slug").and_then(|s| s.as_str()).unwrap_or("");
                            Some(format!("{} | {}", title, slug))
                        })
                        .collect();
                    if let Ok(mut c) = self.crypto.write() {
                        c.events = lines.clone();
                    }
                    self.push_crypto_log(LogLevel::Success, format!("[Crypto] Loaded {} Gamma events", lines.len()));
                } else {
                    self.push_crypto_log(LogLevel::Warning, "[Crypto] Gamma events parse failed".to_string());
                }
            } else {
                self.push_crypto_log(LogLevel::Error, "[Crypto] Gamma request failed".to_string());
            }
        } else {
            self.push_crypto_log(LogLevel::Error, "[Crypto] HTTP client init failed".to_string());
        }

        if let Ok(scraper) = crate::strategies::sports::data::SportsScraper::new() {
            self.push_sports_log(LogLevel::Info, "[Sports] Fetching Premier League fixtures (FBRef)…".to_string());
            match scraper.fetch_pl_fixtures() {
                Ok(fixtures) => {
                    let n = fixtures.len();
                    self.push_sports_log(LogLevel::Success, format!("[Sports] FBRef returned {} fixtures", n));
                    let lines: Vec<String> = fixtures
                        .iter()
                        .take(15)
                        .map(|f| {
                            let score = match (f.home_goals, f.away_goals) {
                                (Some(h), Some(a)) => format!(" {}–{} ", h, a),
                                _ => " – ".to_string(),
                            };
                            format!("{} {}{}{}", f.date, f.home, score, f.away)
                        })
                        .collect();
                    if let Ok(mut s) = self.sports.write() {
                        s.fixtures = lines;
                    }
                    self.push_sports_log(LogLevel::Success, format!("[Sports] Loaded {} Premier League fixtures", n));
                }
                Err(e) => {
                    self.push_sports_log(LogLevel::Error, format!("[Sports] FBRef fetch failed: {}", e));
                }
            }
        } else {
            self.push_sports_log(LogLevel::Error, "[Sports] Scraper init failed (check network)".to_string());
        }

        if let Ok(meteo) = crate::strategies::weather::data::OpenMeteoClient::new() {
            self.push_weather_log(LogLevel::Info, "[Weather] Fetching 7-day forecast (Open-Meteo NYC)…".to_string());
            match meteo.fetch_daily(40.7, -74.0) {
                Ok(daily) => {
                    let lines: Vec<String> = daily
                        .iter()
                        .take(7)
                        .map(|d| {
                            let max = d.temperature_2m_max.map(|t| t.to_string()).unwrap_or_else(|| "—".to_string());
                            let min = d.temperature_2m_min.map(|t| t.to_string()).unwrap_or_else(|| "—".to_string());
                            format!("{}  max {}°C  min {}°C", d.date, max, min)
                        })
                        .collect();
                    if let Ok(mut w) = self.weather.write() {
                        w.forecast = lines.clone();
                    }
                    self.push_weather_log(LogLevel::Success, format!("[Weather] Loaded {} days", lines.len()));
                }
                Err(e) => {
                    self.push_weather_log(LogLevel::Error, format!("[Weather] Open-Meteo fetch failed: {}", e));
                }
            }
        } else {
            self.push_weather_log(LogLevel::Error, "[Weather] Client init failed".to_string());
        }
    }
}
