//! E2E flow tests: simulate key presses via handle_key, then render via TestBackend.
//! Verifies the full chain: key → RouterState mutation → visual output.

use crossterm::event::KeyCode;
use myoro_polymarket_terminal::backtester::BacktesterState;
use myoro_polymarket_terminal::config::Config;
use myoro_polymarket_terminal::copy_trading::TraderList;
use myoro_polymarket_terminal::discover::DiscoverState;
use myoro_polymarket_terminal::live::LiveState;
use myoro_polymarket_terminal::tui::layout::{DiscoverView, Layout, SportsView};
use myoro_polymarket_terminal::tui::router::{handle_key, AppCtx, RouterEffect, RouterState};
use ratatui::backend::TestBackend;
use ratatui::Terminal;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, RwLock};

// ── Shared helpers ────────────────────────────────────────────────────────────

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
        Self {
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

fn press(rs: &mut RouterState, setup: &Setup, code: KeyCode) -> bool {
    matches!(handle_key(rs, code, &setup.ctx()), RouterEffect::Quit)
}

fn make_terminal(w: u16, h: u16) -> Terminal<TestBackend> {
    Terminal::new(TestBackend::new(w, h)).unwrap()
}

fn buf_text(terminal: &Terminal<TestBackend>) -> String {
    let buf = terminal.backend().buffer();
    let area = buf.area;
    (0..area.height)
        .map(|y| {
            (0..area.width)
                .map(|x| buf.cell((x, y)).map_or(" ", |c| c.symbol()).to_string())
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn empty_dv() -> DiscoverView {
    DiscoverView {
        filters_category: "ALL".to_string(),
        filters_period: "WEEK".to_string(),
        filters_order: "P&L".to_string(),
        table: String::new(),
        leaderboard_header: vec![],
        leaderboard_rows: vec![],
        scan_note: String::new(),
        loading: false,
        screener_mode: false,
        screener_markets: vec![],
    }
}

/// Render the current router state into a 120×40 TestBackend buffer.
fn render_state(rs: &RouterState, setup: &Setup) -> String {
    let mut terminal = make_terminal(120, 40);
    let dv = empty_dv();
    let sv: Option<SportsView> = if rs.selected_tab == 1 {
        Some(SportsView::default())
    } else {
        None
    };
    terminal
        .draw(|f| {
            if rs.show_shortcuts_screen {
                Layout::render_shortcuts_screen(f, &[]);
            } else {
                Layout::render(
                    f,
                    rs.selected_tab,
                    "",
                    "",
                    None,
                    None,
                    "",
                    Some(&dv),
                    &setup.live,
                    &[],
                    None,
                    None,
                    None,
                    &rs.scroll_offsets[rs.selected_tab],
                    rs.focused_section,
                    "USD",
                    sv.as_ref(),
                    &setup.backtester,
                    rs.backtest_tool_sel,
                    rs.backtest_strategy_sel,
                    rs.backtest_selected_strategy,
                    rs.backtest_data_sel,
                    rs.backtest_param_sel,
                    rs.backtest_param_editing,
                    &rs.backtest_param_input,
                    rs.backtest_show_graph,
                    rs.backtest_show_help,
                    rs.backtest_data_dialog.as_ref(),
                );
                if let Some(tab) = rs.live_confirm_tab {
                    Layout::render_live_confirm(f, tab);
                }
            }
        })
        .unwrap();
    buf_text(&terminal)
}

// ── Navigation flows ──────────────────────────────────────────────────────────

#[test]
fn flow_press_7_renders_backtester_tab() {
    let setup = Setup::new();
    let mut rs = RouterState::default();
    press(&mut rs, &setup, KeyCode::Char('7'));
    assert_eq!(rs.selected_tab, 6);
    let text = render_state(&rs, &setup);
    assert!(
        text.contains("Strategy") || text.contains("Tools") || text.contains("Backtester"),
        "backtester tab content not found after pressing '7'"
    );
}

#[test]
fn flow_press_1_renders_crypto_tab() {
    let setup = Setup::new();
    let mut rs = RouterState::default();
    rs.selected_tab = 6; // start elsewhere
    press(&mut rs, &setup, KeyCode::Char('1'));
    assert_eq!(rs.selected_tab, 0);
    let text = render_state(&rs, &setup);
    assert!(
        text.contains("Crypto") || text.contains("Strateg"),
        "crypto tab content not found after pressing '1'"
    );
}

#[test]
fn flow_press_2_renders_sports_tab() {
    let setup = Setup::new();
    let mut rs = RouterState::default();
    press(&mut rs, &setup, KeyCode::Char('2'));
    assert_eq!(rs.selected_tab, 1);
    let text = render_state(&rs, &setup);
    assert!(
        text.contains("Signal") || text.contains("Sport"),
        "sports tab content not found after pressing '2'"
    );
}

#[test]
fn flow_e_cycles_through_all_tabs_and_wraps() {
    let setup = Setup::new();
    let mut rs = RouterState::default(); // tab 0
                                         // e×7 should wrap back to tab 0
    for _ in 0..7 {
        press(&mut rs, &setup, KeyCode::Char('e'));
    }
    assert_eq!(
        rs.selected_tab, 0,
        "7 'e' presses should wrap back to tab 0"
    );
    let text = render_state(&rs, &setup);
    assert!(
        text.contains("Crypto") || text.contains("Strateg"),
        "crypto tab not rendered after full wrap"
    );
}

#[test]
fn flow_q_cycles_backward_through_tabs() {
    let setup = Setup::new();
    let mut rs = RouterState::default(); // tab 0
    press(&mut rs, &setup, KeyCode::Char('q')); // wraps to tab 6
    assert_eq!(rs.selected_tab, 6);
    let text = render_state(&rs, &setup);
    assert!(
        text.contains("Strategy") || text.contains("Tools") || text.contains("Backtester"),
        "backtester tab not rendered after 'q' from tab 0"
    );
}

// ── Backtester 5-pane navigation ─────────────────────────────────────────────

#[test]
fn flow_backtester_down_increments_strategy_sel_on_pane_0() {
    let setup = Setup::new();
    let mut rs = RouterState::default();
    rs.selected_tab = 6;
    press(&mut rs, &setup, KeyCode::Down);
    assert_eq!(
        rs.backtest_strategy_sel, 1,
        "Down on pane 0 should increment strategy_sel"
    );
    let text = render_state(&rs, &setup);
    assert!(
        text.contains("Strategy") || text.contains("Tools") || text.contains("Backtester"),
        "backtester tab render broken after Down"
    );
}

#[test]
fn flow_backtester_tab_to_tools_then_down_increments_tool_sel() {
    let setup = Setup::new();
    let mut rs = RouterState::default();
    rs.selected_tab = 6;
    // Tab twice to get to pane 2 (tools)
    press(&mut rs, &setup, KeyCode::Tab);
    press(&mut rs, &setup, KeyCode::Tab);
    assert_eq!(rs.focused_section, 2);
    press(&mut rs, &setup, KeyCode::Down);
    assert_eq!(rs.backtest_tool_sel, 1);
}

#[test]
fn flow_backtester_up_wraps_to_last_and_renders() {
    let setup = Setup::new();
    let mut rs = RouterState::default();
    rs.selected_tab = 6;
    let last_idx = setup.backtester.strategies.len() - 1;
    press(&mut rs, &setup, KeyCode::Up);
    assert_eq!(
        rs.backtest_strategy_sel, last_idx,
        "Up at 0 should wrap to last strategy"
    );
    let text = render_state(&rs, &setup);
    assert!(
        text.contains("Strategy") || text.contains("Backtester"),
        "backtester render broken"
    );
}

#[test]
fn flow_backtester_4_pane_cycle() {
    let setup = Setup::new();
    let mut rs = RouterState::default();
    rs.selected_tab = 6;
    for expected in [1, 2, 3, 0] {
        press(&mut rs, &setup, KeyCode::Tab);
        assert_eq!(rs.focused_section, expected, "pane cycle");
    }
}

// ── Modal flows ───────────────────────────────────────────────────────────────

#[test]
fn flow_question_mark_opens_shortcuts_screen() {
    let setup = Setup::new();
    let mut rs = RouterState::default();
    press(&mut rs, &setup, KeyCode::Char('?'));
    assert!(rs.show_shortcuts_screen, "'?' should open shortcuts screen");
    // render must not panic (shortcuts screen draws a background block with empty shortcuts)
    render_state(&rs, &setup);
}

#[test]
fn flow_esc_in_shortcuts_closes_modal_not_quit() {
    let setup = Setup::new();
    let mut rs = RouterState::default();
    press(&mut rs, &setup, KeyCode::Char('?'));
    assert!(rs.show_shortcuts_screen);
    let quit = press(&mut rs, &setup, KeyCode::Esc); // Esc inside shortcuts → close, not quit
    assert!(!quit, "Esc inside shortcuts should NOT return Quit");
    assert!(
        !rs.show_shortcuts_screen,
        "shortcuts screen should be closed after Esc"
    );
    // After closing, render shows the normal view
    let text = render_state(&rs, &setup);
    assert!(
        text.contains("Crypto") || text.contains("Strateg"),
        "crypto tab not visible after closing shortcuts"
    );
}

#[test]
fn flow_m_on_crypto_opens_live_confirm_overlay() {
    let setup = Setup::new();
    let mut rs = RouterState::default(); // tab 0 = Crypto, Paper mode
    press(&mut rs, &setup, KeyCode::Char('m'));
    assert_eq!(
        rs.live_confirm_tab,
        Some(0),
        "'m' on crypto tab should open live confirm"
    );
    let text = render_state(&rs, &setup);
    assert!(
        text.contains("Live") || text.contains("live") || text.contains("Confirm"),
        "live confirm dialog not rendered"
    );
}

#[test]
fn flow_m_on_backtester_tab_does_nothing() {
    let setup = Setup::new();
    let mut rs = RouterState::default();
    rs.selected_tab = 6;
    press(&mut rs, &setup, KeyCode::Char('m'));
    assert!(
        rs.live_confirm_tab.is_none(),
        "'m' on backtester tab should NOT open live confirm"
    );
}

// ── Tab-switch resets focused section ─────────────────────────────────────────

#[test]
fn flow_tab_switch_resets_focused_section() {
    let setup = Setup::new();
    let mut rs = RouterState::default();
    rs.selected_tab = 3; // copy tab (2 sections)
    press(&mut rs, &setup, KeyCode::Tab); // section 0 → 1
    assert_eq!(rs.focused_section, 1);
    press(&mut rs, &setup, KeyCode::Char('1')); // switch to crypto
    assert_eq!(rs.selected_tab, 0);
    assert_eq!(
        rs.focused_section, 0,
        "focused_section must reset on tab switch"
    );
}
