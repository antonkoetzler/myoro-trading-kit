//! SportsState, StrategyConfig, StoredSignal, and fetch_sports() logic.

use crate::live::global::{push_log_to, LogLevel};
use std::collections::HashMap;
use std::sync::RwLock;

/// Embedded leagues.json — static list of leagues and teams.
const LEAGUES_JSON: &str = include_str!("../sports/leagues.json");

/// A league entry from leagues.json.
#[derive(Clone, Debug, serde::Deserialize)]
pub struct League {
    pub name: String,
    pub short: String,
    pub country: String,
    pub tier: u32,
    pub teams: Vec<String>,
}

impl League {
    pub fn load_all() -> Vec<League> {
        serde_json::from_str(LEAGUES_JSON).unwrap_or_default()
    }
}

/// Configuration for a single strategy (toggle state, auto-execute).
#[derive(Clone, Debug)]
pub struct StrategyConfig {
    pub id: &'static str,
    pub name: &'static str,
    pub description: &'static str,
    pub enabled: bool,
    pub auto_execute: bool,
    pub is_custom: bool,
}

impl StrategyConfig {
    pub fn builtins() -> Vec<StrategyConfig> {
        vec![
            StrategyConfig {
                id: "poisson",
                name: "Poisson Model",
                description: "Poisson + Dixon-Coles. Min edge 5%.",
                enabled: false,
                auto_execute: false,
                is_custom: false,
            },
            StrategyConfig {
                id: "home_adv",
                name: "Home Advantage",
                description: "Elo-adjusted +10% home uplift vs market.",
                enabled: false,
                auto_execute: false,
                is_custom: false,
            },
            StrategyConfig {
                id: "rule_1_20",
                name: "1.20 Rule",
                description: "Value on heavy favs (mkt 0.80-0.87). Min $5 Kelly.",
                enabled: false,
                auto_execute: false,
                is_custom: false,
            },
            StrategyConfig {
                id: "arb_scanner",
                name: "Cross-Platform Arb",
                description: "Poly vs Kalshi price discrepancy detection.",
                enabled: false,
                auto_execute: false,
                is_custom: false,
            },
            StrategyConfig {
                id: "in_play_70min",
                name: "70-Min Tie Rule",
                description: "Late-game xG value for losing team at 65-85 min.",
                enabled: false,
                auto_execute: false,
                is_custom: false,
            },
        ]
    }
}

/// A sports signal as stored in SportsState.
#[derive(Clone, Debug)]
pub struct StoredSignal {
    pub market_id: String,
    pub home: String,
    pub away: String,
    pub date: String,
    pub side: String,
    pub edge_pct: f64,
    pub kelly_size: f64,
    pub strategy_id: String,
    pub status: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// A live match state snapshot for the 70-min rule.
#[derive(Clone, Debug)]
pub struct LiveMatchSnapshot {
    pub home_team: String,
    pub away_team: String,
    pub home_goals: u8,
    pub away_goals: u8,
    pub minute: u8,
}

/// Full sports tab state.
pub struct SportsState {
    pub fixtures: Vec<crate::sports::FixtureWithStats>,
    pub signals: Vec<StoredSignal>,
    pub live_matches: Vec<LiveMatchSnapshot>,
    pub xg_cache: HashMap<String, crate::sports::data::TeamXgStats>,
    pub strategy_configs: Vec<StrategyConfig>,
    pub leagues: Vec<League>,
}

impl Default for SportsState {
    fn default() -> Self {
        Self {
            fixtures: Vec::new(),
            signals: Vec::new(),
            live_matches: Vec::new(),
            xg_cache: HashMap::new(),
            strategy_configs: StrategyConfig::builtins(),
            leagues: League::load_all(),
        }
    }
}

/// Fetch and update all sports data: fixtures, xG, live scores, strategies.
pub fn fetch_sports(sports: &RwLock<SportsState>, sports_logs: &RwLock<Vec<(LogLevel, String)>>) {
    push_log_to(sports_logs, LogLevel::Info, "Fetching fixtures…".into());
    let raw_fixtures = fetch_raw_fixtures(sports_logs);
    let xg_map = fetch_xg_data(sports, sports_logs);
    let live_snapshots = fetch_live_scores();
    let fixtures_with_stats = enrich_fixtures(raw_fixtures, &xg_map, sports_logs);
    let new_signals = run_strategies(&fixtures_with_stats, sports);

    if let Ok(mut s) = sports.write() {
        s.fixtures = fixtures_with_stats;
        s.live_matches = live_snapshots;
        s.xg_cache = xg_map;
        s.signals.extend(new_signals);
        let drain_count = s.signals.len().saturating_sub(200);
        if drain_count > 0 {
            s.signals.drain(0..drain_count);
        }
    }
    push_log_to(sports_logs, LogLevel::Success, "Sports data updated".into());
}

fn fetch_raw_fixtures(
    sports_logs: &RwLock<Vec<(LogLevel, String)>>,
) -> Vec<crate::sports::data::Fixture> {
    match crate::sports::data::SportsScraper::new() {
        Ok(scraper) => match scraper.fetch_pl_fixtures() {
            Ok(f) => {
                push_log_to(
                    sports_logs,
                    LogLevel::Info,
                    format!("FBRef/FixtureDownload: {} PL fixtures", f.len()),
                );
                f
            }
            Err(e) => {
                push_log_to(
                    sports_logs,
                    LogLevel::Warning,
                    format!("Fixture fetch: {}", e),
                );
                Vec::new()
            }
        },
        Err(_) => Vec::new(),
    }
}

fn fetch_xg_data(
    sports: &RwLock<SportsState>,
    sports_logs: &RwLock<Vec<(LogLevel, String)>>,
) -> HashMap<String, crate::sports::data::TeamXgStats> {
    let cached_len = sports.read().map(|s| s.xg_cache.len()).unwrap_or(0);
    if cached_len > 0 {
        return sports
            .read()
            .map(|s| s.xg_cache.clone())
            .unwrap_or_default();
    }
    match crate::sports::data::XgScraper::new() {
        Ok(scraper) => {
            let map = scraper.fetch_pl_xg();
            if !map.is_empty() {
                push_log_to(
                    sports_logs,
                    LogLevel::Info,
                    format!("FBRef xG: {} teams", map.len()),
                );
            }
            map
        }
        Err(_) => HashMap::new(),
    }
}

fn fetch_live_scores() -> Vec<LiveMatchSnapshot> {
    let hour = chrono::Utc::now()
        .format("%H")
        .to_string()
        .parse::<u8>()
        .unwrap_or(0);
    if !(12..=23).contains(&hour) {
        return Vec::new();
    }
    match crate::sports::data::LiveScoresClient::new() {
        Ok(client) => client
            .fetch_live()
            .into_iter()
            .map(|lm| LiveMatchSnapshot {
                home_team: lm.home_team,
                away_team: lm.away_team,
                home_goals: lm.home_goals,
                away_goals: lm.away_goals,
                minute: lm.minute,
            })
            .collect(),
        Err(_) => Vec::new(),
    }
}

fn enrich_fixtures(
    raw: Vec<crate::sports::data::Fixture>,
    xg_map: &HashMap<String, crate::sports::data::TeamXgStats>,
    _sports_logs: &RwLock<Vec<(LogLevel, String)>>,
) -> Vec<crate::sports::FixtureWithStats> {
    let discovery = crate::sports::discovery::MarketDiscovery::new().ok();
    raw.into_iter()
        .map(|f| {
            let mut fws = crate::sports::discovery::FixtureWithStats::from_fixture(f.clone());
            if let Some(home_stats) = xg_map.get(&f.home) {
                fws.home_xg_per_90 = home_stats.xg_per_90.max(0.1);
                fws.home_xga_per_90 = home_stats.xga_per_90.max(0.1);
                fws.home_win_rate = home_stats.home_win_rate;
            }
            if let Some(away_stats) = xg_map.get(&f.away) {
                fws.away_xg_per_90 = away_stats.xg_per_90.max(0.1);
                fws.away_xga_per_90 = away_stats.xga_per_90.max(0.1);
                fws.away_win_rate = away_stats.away_win_rate;
            }
            if let Some(ref disc) = discovery {
                fws.polymarket = disc.find_market(&f);
            }
            fws
        })
        .collect()
}

fn run_strategies(
    fixtures: &[crate::sports::FixtureWithStats],
    sports: &RwLock<SportsState>,
) -> Vec<StoredSignal> {
    let configs = sports
        .read()
        .map(|s| s.strategy_configs.clone())
        .unwrap_or_default();
    let registry = crate::sports::strategies::StrategyRegistry::default();
    let raw_signals = registry.scan(fixtures);
    let enabled_ids: Vec<&str> = configs.iter().filter(|c| c.enabled).map(|c| c.id).collect();

    raw_signals
        .into_iter()
        .filter(|s| enabled_ids.contains(&s.signal.strategy_id.as_str()))
        .map(|s| StoredSignal {
            market_id: s.signal.market_id.clone(),
            home: s.fixture.fixture.home.clone(),
            away: s.fixture.fixture.away.clone(),
            date: s.fixture.fixture.date.clone(),
            side: match s.signal.side {
                crate::shared::strategy::Side::Yes => "YES".to_string(),
                crate::shared::strategy::Side::No => "NO".to_string(),
            },
            edge_pct: s.signal.edge_pct,
            kelly_size: s.signal.kelly_size,
            strategy_id: s.signal.strategy_id.clone(),
            status: "pending".to_string(),
            created_at: s.created_at,
        })
        .collect()
}
