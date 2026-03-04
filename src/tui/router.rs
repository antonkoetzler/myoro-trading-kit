// NOTE: Exceeds 300-line limit — routing logic + 25 inline regression tests must cohere;
// the test suite is the specification for routing priority and cannot be split. See docs/ai-rules/file-size.md
//! Key-routing state machine.
//! Extracts all navigation state + routing logic from runner.rs into a testable unit.

use crate::backtester::BacktesterState;
use crate::config::{Config, ExecutionMode};
use crate::copy_trading::TraderList;
use crate::discover::DiscoverState;
use crate::live::LiveState;
use crate::tui::events::{self, SportsUiState};
use crate::tui::theme::ThemePalette;
use crossterm::event::KeyCode;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, RwLock};

pub const NUM_TABS: usize = 7;
/// Results pane (4) is view-only — not Tab-navigable.
pub const SECTIONS_PER_TAB: [usize; NUM_TABS] = [1, 1, 1, 2, 1, 1, 4];
const PAGE_SCROLL: usize = 10;

// ── Backtester data config dialog ─────────────────────────────────────────────

/// A single editable field in the data-config dialog (text or dropdown).
#[derive(Debug, Clone)]
pub struct ConfigField {
    pub label: String,
    pub value: String,
    /// Non-empty → dropdown; cycling via ←/→.
    pub options: Vec<String>,
}

impl ConfigField {
    pub fn text(label: &str, default: &str) -> Self {
        Self {
            label: label.into(),
            value: default.into(),
            options: vec![],
        }
    }
    pub fn dropdown(label: &str, options: &[&str], default: usize) -> Self {
        Self {
            label: label.into(),
            value: options.get(default).copied().unwrap_or("").to_string(),
            options: options.iter().map(|s| s.to_string()).collect(),
        }
    }
    pub fn is_dropdown(&self) -> bool {
        !self.options.is_empty()
    }
    pub fn cycle_next(&mut self) {
        if let Some(i) = self.options.iter().position(|o| o == &self.value) {
            self.value = self.options[(i + 1) % self.options.len()].clone();
        }
    }
    pub fn cycle_prev(&mut self) {
        if let Some(i) = self.options.iter().position(|o| o == &self.value) {
            let n = self.options.len();
            self.value = self.options[(i + n - 1) % n].clone();
        }
    }
}

/// State for the data-source configuration modal.
#[derive(Debug, Clone)]
pub struct DataConfigDialog {
    pub source_idx: usize,
    pub source_id: String,
    pub source_name: String,
    pub field_sel: usize,
    pub editing: bool,
    pub input: String,
    pub fields: Vec<ConfigField>,
}

impl DataConfigDialog {
    /// Build domain-specific fields for a given data source.
    pub fn build(
        source_idx: usize,
        source_id: &str,
        source_name: &str,
        strategy_domain: &str,
    ) -> Self {
        let fields: Vec<ConfigField> = match source_id {
            "paper" | "synthetic" => vec![],
            "polymarket" => vec![
                ConfigField::text("Market ID / Slug", "will-btc-hit-100k"),
                ConfigField::dropdown("Resolution", &["1m", "5m", "15m", "1h", "1d"], 4),
                ConfigField::text("Start Date", "2024-01-01"),
                ConfigField::text("End Date", "2024-12-31"),
                ConfigField::text("Max Rows", "10000"),
            ],
            "binance" => vec![
                ConfigField::text("Symbol (e.g. BTCUSDT)", "BTCUSDT"),
                ConfigField::dropdown("Interval", &["1m", "5m", "15m", "1h", "4h", "1d"], 5),
                ConfigField::text("Start Date (YYYY-MM-DD)", "2024-01-01"),
                ConfigField::text("End Date (YYYY-MM-DD)", "2024-12-31"),
                ConfigField::text("Max Candles", "1000"),
            ],
            "espn" => {
                let default_sport = match strategy_domain {
                    "sports" => 4usize, // soccer
                    _ => 0,
                };
                vec![
                    ConfigField::dropdown(
                        "Sport",
                        &["nfl", "nba", "mlb", "nhl", "soccer", "tennis", "mma"],
                        default_sport,
                    ),
                    ConfigField::text("League / Competition (e.g. eng.1)", "eng.1"),
                    ConfigField::text("Season / Year", "2024"),
                    ConfigField::text("Team Filter (blank = all)", ""),
                    ConfigField::text("Tournament Filter (blank = all)", ""),
                    ConfigField::text("Start Date (YYYY-MM-DD)", "2024-01-01"),
                    ConfigField::text("End Date (YYYY-MM-DD)", "2024-12-31"),
                ]
            }
            "open_meteo" => vec![
                ConfigField::text("Location (city or lat,lon)", "london"),
                ConfigField::dropdown(
                    "Metric",
                    &["all", "temperature", "precipitation", "wind"],
                    0,
                ),
                ConfigField::dropdown("Resolution", &["daily", "hourly"], 0),
                ConfigField::text("Start Date (YYYY-MM-DD)", "2024-01-01"),
                ConfigField::text("End Date (YYYY-MM-DD)", "2024-12-31"),
            ],
            "import" => vec![
                ConfigField::text("File Path", "data/import/"),
                ConfigField::dropdown("Format", &["auto", "csv", "json"], 0),
                ConfigField::text("Timestamp Column", "timestamp"),
                ConfigField::text("Value Column", "price"),
                ConfigField::text("Separator (CSV)", ","),
            ],
            _ => vec![
                ConfigField::text("Start Date (YYYY-MM-DD)", "2024-01-01"),
                ConfigField::text("End Date (YYYY-MM-DD)", "2024-12-31"),
            ],
        };
        Self {
            source_idx,
            source_id: source_id.into(),
            source_name: source_name.into(),
            field_sel: 0,
            editing: false,
            input: String::new(),
            fields,
        }
    }
}

// ── RouterState ───────────────────────────────────────────────────────────────

pub struct RouterState {
    pub selected_tab: usize,
    pub focused_section: usize,
    pub scroll_offsets: Vec<Vec<usize>>,
    pub backtest_tool_sel: usize,
    pub backtest_strategy_sel: usize,
    pub backtest_data_sel: usize,
    pub backtest_param_sel: usize,
    pub backtest_param_editing: bool,
    pub backtest_param_input: String,
    /// Confirmed strategy selection (None = none selected yet).
    pub backtest_selected_strategy: Option<usize>,
    /// Full-screen graph expansion overlay.
    pub backtest_show_graph: bool,
    /// Backtester-specific help overlay.
    pub backtest_show_help: bool,
    /// Data-source configuration dialog.
    pub backtest_data_dialog: Option<DataConfigDialog>,
    pub crypto_strategy_sel: usize,
    pub weather_strategy_sel: usize,
    pub sports_ui: SportsUiState,
    pub copy_selected: Option<usize>,
    pub copy_add_dialog: Option<(String, usize)>,
    pub discover_selected: Option<usize>,
    pub show_shortcuts_screen: bool,
    pub show_theme_overlay: bool,
    pub theme_overlay_sel: usize,
    pub theme_in_creator: bool,
    pub theme_creator_role: usize,
    pub theme_creator_color_idx: usize,
    pub theme_editor_palette: Option<ThemePalette>,
    pub live_confirm_tab: Option<usize>,
    pub bankroll_input: Option<String>,
    pub show_currency_picker: bool,
    pub currency_filter: String,
    pub currency_selected: usize,
    pub discover_filter_dialog: Option<events::discover::DiscoverFilterDialog>,
    pub tab_execution_mode: [ExecutionMode; NUM_TABS],
}

impl Default for RouterState {
    fn default() -> Self {
        Self {
            selected_tab: 0,
            focused_section: 0,
            scroll_offsets: vec![
                vec![0],
                vec![0],
                vec![0],
                vec![0, 0],
                vec![0],
                vec![0],
                vec![0, 0, 0, 0, 0],
            ],
            backtest_tool_sel: 0,
            backtest_strategy_sel: 0,
            backtest_data_sel: 0,
            backtest_param_sel: 0,
            backtest_param_editing: false,
            backtest_param_input: String::new(),
            backtest_selected_strategy: None,
            backtest_show_graph: false,
            backtest_show_help: false,
            backtest_data_dialog: None,
            crypto_strategy_sel: 0,
            weather_strategy_sel: 0,
            sports_ui: SportsUiState::default(),
            copy_selected: None,
            copy_add_dialog: None,
            discover_selected: None,
            show_shortcuts_screen: false,
            show_theme_overlay: false,
            theme_overlay_sel: 0,
            theme_in_creator: false,
            theme_creator_role: 0,
            theme_creator_color_idx: 0,
            theme_editor_palette: None,
            live_confirm_tab: None,
            bankroll_input: None,
            show_currency_picker: false,
            currency_filter: String::new(),
            currency_selected: 0,
            discover_filter_dialog: None,
            tab_execution_mode: [ExecutionMode::Paper; NUM_TABS],
        }
    }
}

// ── RouterEffect + AppCtx ─────────────────────────────────────────────────────

pub enum RouterEffect {
    Continue,
    Quit,
}

pub struct AppCtx<'a> {
    pub live: &'a LiveState,
    pub trader_list: &'a Arc<TraderList>,
    pub discover: &'a Arc<DiscoverState>,
    pub copy_running: &'a AtomicBool,
    pub config: &'a Arc<RwLock<Config>>,
    pub backtester: &'a Arc<BacktesterState>,
}

// ── handle_key ────────────────────────────────────────────────────────────────

/// Route one keypress. Pure state transform + side-effect dispatcher; never blocks.
/// Returns `RouterEffect::Quit` only for Esc outside all modals.
pub fn handle_key(state: &mut RouterState, code: KeyCode, ctx: &AppCtx) -> RouterEffect {
    // ── Modal chain (highest priority) ────────────────────────────────────────

    // Backtester data-config dialog
    if let Some(ref mut dialog) = state.backtest_data_dialog {
        match code {
            KeyCode::Esc => {
                if dialog.editing {
                    dialog.editing = false;
                    dialog.input.clear();
                } else {
                    state.backtest_data_dialog = None;
                }
            }
            KeyCode::Enter if dialog.fields.is_empty() => {
                state.backtest_data_dialog = None;
                state.focused_section = 2;
            }
            KeyCode::Enter if !dialog.editing => {
                if !dialog.fields[dialog.field_sel].is_dropdown() {
                    let val = dialog.fields[dialog.field_sel].value.clone();
                    dialog.editing = true;
                    dialog.input = val;
                }
                // dropdowns use ←/→
            }
            KeyCode::Enter if dialog.editing => {
                let val = dialog.input.clone();
                dialog.fields[dialog.field_sel].value = val;
                dialog.editing = false;
                dialog.input.clear();
            }
            KeyCode::Char('s') if !dialog.editing => {
                // [s] = save and close dialog, advance to Tool pane
                state.backtest_data_dialog = None;
                state.focused_section = 2;
            }
            KeyCode::Up if !dialog.editing => {
                dialog.field_sel = dialog.field_sel.saturating_sub(1);
            }
            KeyCode::Down if !dialog.editing => {
                if !dialog.fields.is_empty() {
                    dialog.field_sel = (dialog.field_sel + 1).min(dialog.fields.len() - 1);
                }
            }
            KeyCode::Left if !dialog.editing => {
                let fsel = dialog.field_sel;
                if let Some(f) = dialog.fields.get_mut(fsel) {
                    if f.is_dropdown() {
                        f.cycle_prev();
                    }
                }
            }
            KeyCode::Right if !dialog.editing => {
                let fsel = dialog.field_sel;
                if let Some(f) = dialog.fields.get_mut(fsel) {
                    if f.is_dropdown() {
                        f.cycle_next();
                    }
                }
            }
            KeyCode::Char(c) if dialog.editing => {
                dialog.input.push(c);
            }
            KeyCode::Backspace if dialog.editing => {
                dialog.input.pop();
            }
            _ => {}
        }
        return RouterEffect::Continue;
    }

    // Backtester graph overlay (dismiss with any key)
    if state.backtest_show_graph {
        state.backtest_show_graph = false;
        return RouterEffect::Continue;
    }

    // Backtester help overlay
    if state.backtest_show_help {
        if matches!(code, KeyCode::Esc | KeyCode::Char('H') | KeyCode::Char('?')) {
            state.backtest_show_help = false;
        }
        return RouterEffect::Continue;
    }

    if state.live_confirm_tab.is_some() {
        events::global::handle_live_confirm(
            code,
            &mut state.live_confirm_tab,
            &mut state.tab_execution_mode,
        );
        return RouterEffect::Continue;
    }

    if let Some(ref mut inner) = state.bankroll_input.take() {
        let mut s = inner.clone();
        // sentinel: None after Esc/Enter, Some after Backspace/Char
        let mut sentinel: Option<String> = Some(String::new());
        events::global::handle_bankroll_input(code, &mut s, ctx.live, ctx.config, &mut sentinel);
        if sentinel.is_some() {
            state.bankroll_input = Some(s);
        }
        return RouterEffect::Continue;
    }

    if state.show_shortcuts_screen {
        if matches!(code, KeyCode::Char('?') | KeyCode::Esc) {
            state.show_shortcuts_screen = false;
        }
        return RouterEffect::Continue;
    }

    if events::discover::handle_filter_dialog_key(
        code,
        &mut state.discover_filter_dialog,
        ctx.discover,
    ) {
        return RouterEffect::Continue;
    }

    if state.show_currency_picker
        && events::global::handle_currency_picker(
            code,
            &mut state.currency_filter,
            &mut state.currency_selected,
            &mut state.show_currency_picker,
            ctx.config,
        )
    {
        return RouterEffect::Continue;
    }

    if state.show_theme_overlay
        && events::global::handle_theme_overlay(
            code,
            &mut state.show_theme_overlay,
            &mut state.theme_overlay_sel,
            &mut state.theme_in_creator,
            &mut state.theme_creator_role,
            &mut state.theme_creator_color_idx,
            &mut state.theme_editor_palette,
        )
    {
        return RouterEffect::Continue;
    }

    let discover_entries = ctx.discover.get_entries();
    if state.copy_add_dialog.is_some()
        && events::copy::handle_add_dialog_key(
            code,
            &mut state.copy_add_dialog,
            &discover_entries,
            ctx.trader_list,
            ctx.live,
        )
    {
        return RouterEffect::Continue;
    }

    // ── Global key routing ────────────────────────────────────────────────────
    match code {
        KeyCode::Char('?') => state.show_shortcuts_screen = true,
        KeyCode::Char('T') | KeyCode::F(10) => state.show_theme_overlay = true,
        KeyCode::Char('b') => state.bankroll_input = Some(String::new()),
        KeyCode::Char('C') => {
            state.show_currency_picker = true;
            state.currency_filter.clear();
            state.currency_selected = 0;
        }
        KeyCode::Esc => return RouterEffect::Quit,
        KeyCode::Char('q') => {
            state.selected_tab = if state.selected_tab == 0 {
                NUM_TABS - 1
            } else {
                state.selected_tab - 1
            };
            state.focused_section = 0;
        }
        KeyCode::Char('e') => {
            state.selected_tab = if state.selected_tab + 1 >= NUM_TABS {
                0
            } else {
                state.selected_tab + 1
            };
            state.focused_section = 0;
        }
        KeyCode::Left | KeyCode::Char('h') => {
            if state.selected_tab == 1 {
                state.sports_ui.pane = state.sports_ui.pane.saturating_sub(1);
            } else if state.focused_section < SECTIONS_PER_TAB[state.selected_tab] {
                let offs = &mut state.scroll_offsets[state.selected_tab][state.focused_section];
                *offs = offs.saturating_sub(PAGE_SCROLL);
            }
        }
        KeyCode::Right | KeyCode::Char('l') => {
            if state.selected_tab == 1 {
                state.sports_ui.pane = (state.sports_ui.pane + 1).min(2);
            } else if state.focused_section < SECTIONS_PER_TAB[state.selected_tab] {
                let offs = &mut state.scroll_offsets[state.selected_tab][state.focused_section];
                *offs = offs.saturating_add(PAGE_SCROLL);
            }
        }
        KeyCode::Up | KeyCode::Char('k') => {
            if state.selected_tab == 1 {
                match state.sports_ui.pane {
                    0 => {
                        state.sports_ui.strategy_sel =
                            state.sports_ui.strategy_sel.saturating_sub(1)
                    }
                    1 => state.sports_ui.signal_sel = state.sports_ui.signal_sel.saturating_sub(1),
                    _ => {
                        state.scroll_offsets[1][0] = state.scroll_offsets[1][0].saturating_sub(1);
                        state.sports_ui.fixture_sel = state.sports_ui.fixture_sel.saturating_sub(1);
                    }
                }
            } else if state.selected_tab == 6 {
                // Backtester 5-pane navigation with circular wrap
                let strat_len = ctx.backtester.strategies.len();
                let data_len = ctx.backtester.data_sources.len();
                let tool_len = crate::backtester::BacktestTool::all().len();
                match state.focused_section {
                    0 => {
                        state.backtest_strategy_sel =
                            if strat_len > 0 && state.backtest_strategy_sel == 0 {
                                strat_len - 1
                            } else {
                                state.backtest_strategy_sel.saturating_sub(1)
                            }
                    }
                    1 => {
                        state.backtest_data_sel = if data_len > 0 && state.backtest_data_sel == 0 {
                            data_len - 1
                        } else {
                            state.backtest_data_sel.saturating_sub(1)
                        }
                    }
                    2 => {
                        state.backtest_tool_sel = if tool_len > 0 && state.backtest_tool_sel == 0 {
                            tool_len - 1
                        } else {
                            state.backtest_tool_sel.saturating_sub(1)
                        }
                    }
                    3 => state.backtest_param_sel = state.backtest_param_sel.saturating_sub(1),
                    _ => {
                        let offs = &mut state.scroll_offsets[6][4];
                        *offs = offs.saturating_sub(1);
                    }
                }
            } else if state.focused_section < SECTIONS_PER_TAB[state.selected_tab] {
                let offs = &mut state.scroll_offsets[state.selected_tab][state.focused_section];
                *offs = offs.saturating_sub(1);
            }
        }
        KeyCode::Down | KeyCode::Char('j') => {
            if state.selected_tab == 1 {
                match state.sports_ui.pane {
                    0 => state.sports_ui.strategy_sel += 1,
                    1 => state.sports_ui.signal_sel += 1,
                    _ => {
                        state.scroll_offsets[1][0] += 1;
                        state.sports_ui.fixture_sel += 1;
                    }
                }
            } else if state.selected_tab == 6 {
                // Backtester 5-pane navigation with circular wrap and dynamic bounds
                let strat_len = ctx.backtester.strategies.len();
                let data_len = ctx.backtester.data_sources.len();
                let tool_len = crate::backtester::BacktestTool::all().len();
                match state.focused_section {
                    0 => {
                        state.backtest_strategy_sel = if strat_len > 0 {
                            (state.backtest_strategy_sel + 1) % strat_len
                        } else {
                            0
                        }
                    }
                    1 => {
                        state.backtest_data_sel = if data_len > 0 {
                            (state.backtest_data_sel + 1) % data_len
                        } else {
                            0
                        }
                    }
                    2 => {
                        state.backtest_tool_sel = if tool_len > 0 {
                            (state.backtest_tool_sel + 1) % tool_len
                        } else {
                            0
                        }
                    }
                    3 => state.backtest_param_sel += 1,
                    _ => {
                        let offs = &mut state.scroll_offsets[6][4];
                        *offs += 1;
                    }
                }
            } else if state.focused_section < SECTIONS_PER_TAB[state.selected_tab] {
                let offs = &mut state.scroll_offsets[state.selected_tab][state.focused_section];
                *offs += 1;
            }
        }
        // Backtester: Enter on strategy pane confirms selection, advances to data pane
        KeyCode::Enter
            if state.selected_tab == 6
                && state.focused_section == 0
                && !state.backtest_param_editing =>
        {
            state.backtest_selected_strategy = Some(state.backtest_strategy_sel);
            // Reset data selection when strategy changes
            state.backtest_data_sel = 0;
            state.focused_section = 1;
        }
        // Backtester: Enter on data pane opens config dialog
        KeyCode::Enter
            if state.selected_tab == 6
                && state.focused_section == 1
                && !state.backtest_param_editing =>
        {
            let data_sel = state.backtest_data_sel;
            let strategy_domain = state
                .backtest_selected_strategy
                .and_then(|idx| ctx.backtester.strategies.get(idx))
                .map(|s| s.domain.as_str())
                .unwrap_or("all");
            if let Some(source) = ctx.backtester.data_sources.get(data_sel) {
                state.backtest_data_dialog = Some(DataConfigDialog::build(
                    data_sel,
                    &source.id.clone(),
                    &source.name.clone(),
                    strategy_domain,
                ));
            }
        }
        // Backtester: Enter on tool pane advances to params pane
        KeyCode::Enter
            if state.selected_tab == 6
                && state.focused_section == 2
                && !state.backtest_param_editing =>
        {
            state.focused_section = 3;
        }
        // Backtester: g = toggle full-screen graph
        KeyCode::Char('g') if state.selected_tab == 6 && !state.backtest_param_editing => {
            state.backtest_show_graph = !state.backtest_show_graph;
        }
        // Backtester: H = toggle backtester help
        KeyCode::Char('H') if state.selected_tab == 6 => {
            state.backtest_show_help = !state.backtest_show_help;
        }
        KeyCode::Tab => {
            state.focused_section =
                (state.focused_section + 1) % SECTIONS_PER_TAB[state.selected_tab];
        }
        KeyCode::BackTab => {
            let n = SECTIONS_PER_TAB[state.selected_tab];
            state.focused_section = (state.focused_section + n - 1) % n;
        }
        KeyCode::Char('1') => {
            state.selected_tab = 0;
            state.focused_section = 0;
        }
        KeyCode::Char('2') => {
            state.selected_tab = 1;
            state.focused_section = 0;
        }
        KeyCode::Char('3') => {
            state.selected_tab = 2;
            state.focused_section = 0;
        }
        KeyCode::Char('4') => {
            state.selected_tab = 3;
            state.focused_section = 0;
        }
        KeyCode::Char('5') => {
            state.selected_tab = 4;
            state.focused_section = 0;
        }
        KeyCode::Char('6') => {
            state.selected_tab = 5;
            state.focused_section = 0;
        }
        KeyCode::Char('7') => {
            state.selected_tab = 6;
            state.focused_section = 0;
        }
        KeyCode::Char('m') => {
            if state.selected_tab < 4 {
                if state.tab_execution_mode[state.selected_tab] == ExecutionMode::Paper {
                    state.live_confirm_tab = Some(state.selected_tab);
                } else {
                    state.tab_execution_mode[state.selected_tab] = ExecutionMode::Paper;
                }
            }
        }
        _ => match state.selected_tab {
            0 => events::crypto::handle_key(code, &mut state.crypto_strategy_sel, ctx.live),
            1 => events::sports::handle_key(
                code,
                &mut state.sports_ui,
                ctx.live,
                &mut state.scroll_offsets[1][0],
            ),
            2 => events::weather::handle_key(code, &mut state.weather_strategy_sel, ctx.live),
            3 => events::copy::handle_key(
                code,
                ctx.copy_running,
                &mut state.copy_selected,
                &mut state.copy_add_dialog,
                ctx.trader_list,
                ctx.trader_list.len(),
                ctx.live,
            ),
            4 => {}
            5 => events::discover::handle_key(
                code,
                ctx.discover,
                state.discover_selected,
                ctx.trader_list,
                &discover_entries,
                ctx.live,
                &mut state.discover_filter_dialog,
            ),
            6 => {
                events::backtester::handle_key(
                    code,
                    ctx.backtester,
                    state.backtest_strategy_sel,
                    state.backtest_data_sel,
                    state.backtest_tool_sel,
                    state.backtest_param_sel,
                    &mut state.backtest_param_editing,
                    &mut state.backtest_param_input,
                    state.focused_section,
                );
            }
            _ => {}
        },
    }
    RouterEffect::Continue
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::Ordering;

    struct Setup {
        live: LiveState,
        config: Arc<RwLock<Config>>,
        trader_list: Arc<TraderList>,
        discover: Arc<DiscoverState>,
        copy_running: AtomicBool,
        backtester: Arc<BacktesterState>,
    }

    impl Setup {
        fn new() -> Self {
            let config = Arc::new(RwLock::new(Config::default()));
            let trader_list = Arc::new(TraderList::new(Arc::clone(&config)));
            Setup {
                live: LiveState::default(),
                config,
                trader_list,
                discover: Arc::new(DiscoverState::new()),
                copy_running: AtomicBool::new(false),
                backtester: BacktesterState::new(),
            }
        }
        fn ctx(&self) -> AppCtx<'_> {
            AppCtx {
                live: &self.live,
                trader_list: &self.trader_list,
                discover: &self.discover,
                copy_running: &self.copy_running,
                config: &self.config,
                backtester: &self.backtester,
            }
        }
    }

    fn press(state: &mut RouterState, setup: &Setup, code: KeyCode) -> bool {
        matches!(handle_key(state, code, &setup.ctx()), RouterEffect::Quit)
    }

    // ── Regression: the exact bug we hit ──────────────────────────────────────

    // Backtester 5-pane navigation tests

    #[test]
    fn up_on_backtester_pane_0_decrements_strategy_sel() {
        let s = Setup::new();
        let mut st = RouterState::default();
        st.selected_tab = 6;
        st.backtest_strategy_sel = 3;
        press(&mut st, &s, KeyCode::Up);
        assert_eq!(st.backtest_strategy_sel, 2);
    }

    #[test]
    fn down_on_backtester_pane_2_increments_tool_sel() {
        let s = Setup::new();
        let mut st = RouterState::default();
        st.selected_tab = 6;
        st.focused_section = 2;
        press(&mut st, &s, KeyCode::Down);
        assert_eq!(st.backtest_tool_sel, 1);
    }

    #[test]
    fn down_on_backtester_pane_2_wraps_at_last() {
        let s = Setup::new();
        let mut st = RouterState::default();
        st.selected_tab = 6;
        st.focused_section = 2;
        let last = crate::backtester::BacktestTool::all().len() - 1;
        st.backtest_tool_sel = last;
        press(&mut st, &s, KeyCode::Down);
        assert_eq!(st.backtest_tool_sel, 0);
    }

    #[test]
    fn up_on_backtester_pane_1_decrements_data_sel() {
        let s = Setup::new();
        let mut st = RouterState::default();
        st.selected_tab = 6;
        st.focused_section = 1;
        st.backtest_data_sel = 2;
        press(&mut st, &s, KeyCode::Up);
        assert_eq!(st.backtest_data_sel, 1);
    }

    #[test]
    fn backtester_tab_cycles_4_panes() {
        let s = Setup::new();
        let mut st = RouterState::default();
        st.selected_tab = 6;
        for expected in [1, 2, 3, 0] {
            press(&mut st, &s, KeyCode::Tab);
            assert_eq!(st.focused_section, expected, "tab cycle pane");
        }
    }

    // ── Tab navigation ────────────────────────────────────────────────────────

    #[test]
    fn chars_1_through_7_switch_tabs() {
        let s = Setup::new();
        for (ch, expected) in [
            ('1', 0),
            ('2', 1),
            ('3', 2),
            ('4', 3),
            ('5', 4),
            ('6', 5),
            ('7', 6),
        ] {
            let mut st = RouterState::default();
            press(&mut st, &s, KeyCode::Char(ch));
            assert_eq!(
                st.selected_tab, expected,
                "char '{}' → tab {}",
                ch, expected
            );
        }
    }

    #[test]
    fn e_key_cycles_forward_with_wrap() {
        let s = Setup::new();
        let mut st = RouterState::default();
        st.selected_tab = 6;
        press(&mut st, &s, KeyCode::Char('e'));
        assert_eq!(st.selected_tab, 0);
    }

    #[test]
    fn q_key_cycles_backward_with_wrap() {
        let s = Setup::new();
        let mut st = RouterState::default();
        st.selected_tab = 0;
        press(&mut st, &s, KeyCode::Char('q'));
        assert_eq!(st.selected_tab, 6);
    }

    #[test]
    fn tab_switch_resets_focused_section() {
        let s = Setup::new();
        let mut st = RouterState::default();
        st.selected_tab = 3;
        st.focused_section = 1;
        press(&mut st, &s, KeyCode::Char('1'));
        assert_eq!(st.focused_section, 0);
    }

    // ── Section focus ─────────────────────────────────────────────────────────

    #[test]
    fn tab_key_cycles_sections() {
        let s = Setup::new();
        let mut st = RouterState::default();
        st.selected_tab = 3; // 2 sections
        assert_eq!(st.focused_section, 0);
        press(&mut st, &s, KeyCode::Tab);
        assert_eq!(st.focused_section, 1);
        press(&mut st, &s, KeyCode::Tab);
        assert_eq!(st.focused_section, 0);
    }

    #[test]
    fn back_tab_cycles_sections_backward() {
        let s = Setup::new();
        let mut st = RouterState::default();
        st.selected_tab = 3;
        st.focused_section = 0;
        press(&mut st, &s, KeyCode::BackTab);
        assert_eq!(st.focused_section, 1);
    }

    #[test]
    fn tab_key_on_single_section_tab_stays_zero() {
        let s = Setup::new();
        let mut st = RouterState::default();
        st.selected_tab = 0; // 1 section
        press(&mut st, &s, KeyCode::Tab);
        assert_eq!(st.focused_section, 0);
    }

    // ── Scrolling ─────────────────────────────────────────────────────────────

    #[test]
    fn up_on_non_special_tab_decrements_scroll() {
        let s = Setup::new();
        let mut st = RouterState::default();
        st.selected_tab = 0;
        st.scroll_offsets[0][0] = 5;
        press(&mut st, &s, KeyCode::Up);
        assert_eq!(st.scroll_offsets[0][0], 4);
    }

    #[test]
    fn up_scroll_saturates_at_zero() {
        let s = Setup::new();
        let mut st = RouterState::default();
        st.selected_tab = 0;
        st.scroll_offsets[0][0] = 0;
        press(&mut st, &s, KeyCode::Up);
        assert_eq!(st.scroll_offsets[0][0], 0);
    }

    #[test]
    fn down_on_non_special_tab_increments_scroll() {
        let s = Setup::new();
        let mut st = RouterState::default();
        st.selected_tab = 0;
        st.scroll_offsets[0][0] = 3;
        press(&mut st, &s, KeyCode::Down);
        assert_eq!(st.scroll_offsets[0][0], 4);
    }

    // ── Sports routing ────────────────────────────────────────────────────────

    #[test]
    fn right_key_advances_sports_pane() {
        let s = Setup::new();
        let mut st = RouterState::default();
        st.selected_tab = 1;
        st.sports_ui.pane = 0;
        press(&mut st, &s, KeyCode::Right);
        assert_eq!(st.sports_ui.pane, 1);
        press(&mut st, &s, KeyCode::Right);
        assert_eq!(st.sports_ui.pane, 2);
        press(&mut st, &s, KeyCode::Right); // clamp
        assert_eq!(st.sports_ui.pane, 2);
    }

    #[test]
    fn left_key_decrements_sports_pane() {
        let s = Setup::new();
        let mut st = RouterState::default();
        st.selected_tab = 1;
        st.sports_ui.pane = 2;
        press(&mut st, &s, KeyCode::Left);
        assert_eq!(st.sports_ui.pane, 1);
        press(&mut st, &s, KeyCode::Left);
        assert_eq!(st.sports_ui.pane, 0);
        press(&mut st, &s, KeyCode::Left); // saturate
        assert_eq!(st.sports_ui.pane, 0);
    }

    #[test]
    fn up_sports_pane_0_decrements_strategy_sel() {
        let s = Setup::new();
        let mut st = RouterState::default();
        st.selected_tab = 1;
        st.sports_ui.pane = 0;
        st.sports_ui.strategy_sel = 2;
        press(&mut st, &s, KeyCode::Up);
        assert_eq!(st.sports_ui.strategy_sel, 1);
    }

    #[test]
    fn up_sports_pane_1_decrements_signal_sel() {
        let s = Setup::new();
        let mut st = RouterState::default();
        st.selected_tab = 1;
        st.sports_ui.pane = 1;
        st.sports_ui.signal_sel = 3;
        press(&mut st, &s, KeyCode::Up);
        assert_eq!(st.sports_ui.signal_sel, 2);
    }

    #[test]
    fn down_sports_pane_0_increments_strategy_sel() {
        let s = Setup::new();
        let mut st = RouterState::default();
        st.selected_tab = 1;
        st.sports_ui.pane = 0;
        st.sports_ui.strategy_sel = 1;
        press(&mut st, &s, KeyCode::Down);
        assert_eq!(st.sports_ui.strategy_sel, 2);
    }

    // ── Modals ────────────────────────────────────────────────────────────────

    #[test]
    fn shortcuts_screen_blocks_tab_switch() {
        let s = Setup::new();
        let mut st = RouterState::default();
        st.selected_tab = 0;
        st.show_shortcuts_screen = true;
        press(&mut st, &s, KeyCode::Char('7')); // would switch to tab 6 normally
        assert_eq!(
            st.selected_tab, 0,
            "tab must not change while shortcuts open"
        );
        // '7' is not '?' or Esc, so shortcuts screen stays open
        assert!(
            st.show_shortcuts_screen,
            "'7' does not close the shortcuts screen"
        );
    }

    #[test]
    fn shortcuts_screen_closed_by_question_mark() {
        let s = Setup::new();
        let mut st = RouterState::default();
        st.show_shortcuts_screen = true;
        press(&mut st, &s, KeyCode::Char('?'));
        assert!(!st.show_shortcuts_screen);
    }

    #[test]
    fn m_key_on_trading_tab_opens_live_confirm() {
        let s = Setup::new();
        let mut st = RouterState::default();
        st.selected_tab = 0;
        press(&mut st, &s, KeyCode::Char('m'));
        assert_eq!(st.live_confirm_tab, Some(0));
    }

    #[test]
    fn m_key_on_backtester_tab_does_nothing() {
        let s = Setup::new();
        let mut st = RouterState::default();
        st.selected_tab = 6;
        press(&mut st, &s, KeyCode::Char('m'));
        assert!(st.live_confirm_tab.is_none());
    }

    #[test]
    fn esc_outside_modal_returns_quit() {
        let s = Setup::new();
        let mut st = RouterState::default();
        let quit = press(&mut st, &s, KeyCode::Esc);
        assert!(quit);
    }

    #[test]
    fn esc_inside_shortcuts_closes_not_quit() {
        let s = Setup::new();
        let mut st = RouterState::default();
        st.show_shortcuts_screen = true;
        let quit = press(&mut st, &s, KeyCode::Esc);
        assert!(!quit, "Esc inside shortcuts must not quit");
        assert!(!st.show_shortcuts_screen);
    }

    // ── Bankroll input ────────────────────────────────────────────────────────

    #[test]
    fn bankroll_input_b_key_opens_prompt() {
        let s = Setup::new();
        let mut st = RouterState::default();
        press(&mut st, &s, KeyCode::Char('b'));
        assert_eq!(st.bankroll_input, Some(String::new()));
    }

    #[test]
    fn bankroll_input_digit_is_stored() {
        let s = Setup::new();
        let mut st = RouterState::default();
        st.bankroll_input = Some(String::new());
        press(&mut st, &s, KeyCode::Char('5'));
        assert_eq!(st.bankroll_input, Some("5".to_string()));
    }

    #[test]
    fn bankroll_input_esc_clears() {
        let s = Setup::new();
        let mut st = RouterState::default();
        st.bankroll_input = Some("123".to_string());
        press(&mut st, &s, KeyCode::Esc);
        assert!(st.bankroll_input.is_none());
    }

    #[test]
    fn live_confirm_y_sets_live_mode() {
        let s = Setup::new();
        let mut st = RouterState::default();
        st.live_confirm_tab = Some(0);
        press(&mut st, &s, KeyCode::Char('y'));
        assert!(st.live_confirm_tab.is_none());
        assert_eq!(st.tab_execution_mode[0], ExecutionMode::Live);
    }

    #[test]
    fn live_confirm_n_cancels() {
        let s = Setup::new();
        let mut st = RouterState::default();
        st.live_confirm_tab = Some(2);
        press(&mut st, &s, KeyCode::Char('n'));
        assert!(st.live_confirm_tab.is_none());
        assert_eq!(
            st.tab_execution_mode[2],
            ExecutionMode::Paper,
            "mode must stay Paper on cancel"
        );
    }

    #[test]
    fn copy_running_copy_running_state_is_readable() {
        let s = Setup::new();
        assert!(!s.copy_running.load(Ordering::SeqCst));
    }
}
