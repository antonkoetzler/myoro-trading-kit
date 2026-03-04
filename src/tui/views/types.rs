//! Shared view-model types used by layout.rs and per-tab view modules.

/// A strategy row in the Sports strategies pane.
#[derive(Clone)]
pub struct StrategyRow {
    pub id: String,
    pub name: String,
    pub enabled: bool,
    pub is_custom: bool,
    pub selected: bool,
}

/// A signal row in the Sports signals pane.
#[derive(Clone)]
pub struct SignalRow {
    pub team: String,
    pub side: String,
    pub edge_pct: f64,
    pub kelly_size: f64,
    pub status: String,
    pub strategy_id: String,
    pub selected: bool,
}

/// A fixture row in the Sports fixtures pane.
#[derive(Clone)]
pub struct FixtureRow {
    pub date: String,
    pub home: String,
    pub away: String,
    pub has_market: bool,
    pub selected: bool,
    pub is_date_header: bool,
}

/// Full sports 3-pane view model.
#[derive(Clone, Default)]
pub struct SportsView {
    pub pane: usize,
    pub strategies: Vec<StrategyRow>,
    pub signals: Vec<SignalRow>,
    pub fixtures: Vec<FixtureRow>,
    pub pending_count: usize,
    pub league_filter: Option<String>,
    pub team_filter: Option<String>,
    pub show_league_picker: bool,
    pub show_team_picker: bool,
    pub league_picker_items: Vec<String>,
    pub team_picker_items: Vec<String>,
    pub league_picker_sel: usize,
    pub team_picker_sel: usize,
}

/// Discover tab view model.
#[derive(Clone)]
pub struct DiscoverView {
    pub filters_category: String,
    pub filters_period: String,
    pub filters_order: String,
    pub table: String,
    pub leaderboard_header: Vec<String>,
    pub leaderboard_rows: Vec<(bool, bool, bool, Vec<String>)>,
    pub scan_note: String,
    pub loading: bool,
    pub screener_mode: bool,
    pub screener_markets: Vec<crate::discover::ScreenerMarket>,
}

/// Tab names, in order.
pub const TABS: &[&str] = &[
    "Crypto",
    "Sports",
    "Weather",
    "Copy",
    "Portfolio",
    "Discover",
    "Backtester",
];

/// A (key, description) pair for the shortcuts screen.
pub type ShortcutPair = (String, String);
