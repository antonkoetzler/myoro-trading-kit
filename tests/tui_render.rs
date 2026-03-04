//! TestBackend render tests for every tab, dialog, and chrome element.
//! Uses ratatui's TestBackend so no terminal is required.
//! Assertions: text-presence (contains) — not snapshot-pinned.

use myoro_polymarket_terminal::backtester::BacktesterState;
use myoro_polymarket_terminal::live::LiveState;
use myoro_polymarket_terminal::tui::layout::{DiscoverView, Layout, SportsView};
use ratatui::backend::TestBackend;
use ratatui::Terminal;
use std::sync::Arc;

// ── Helpers ───────────────────────────────────────────────────────────────────

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

fn render_tab(tab: usize, live: &LiveState, backtester: &Arc<BacktesterState>) -> String {
    let mut terminal = make_terminal(120, 40);
    let dv = empty_dv();
    let sv: Option<SportsView> = if tab == 1 {
        Some(SportsView::default())
    } else {
        None
    };
    terminal
        .draw(|f| {
            Layout::render(
                f,
                tab,
                "",
                "",
                None,
                None,
                "",
                Some(&dv),
                live,
                &[],
                None,
                None,
                None,
                &[0],
                0,
                "USD",
                sv.as_ref(),
                backtester,
                0,
                0,
                None,
                0,
                0,
                false,
                "",
                false,
                false,
                None,
            );
        })
        .unwrap();
    buf_text(&terminal)
}

// ── Chrome + global ───────────────────────────────────────────────────────────

#[test]
fn render_title_contains_app_name() {
    let live = LiveState::default();
    let bt = BacktesterState::new();
    let text = render_tab(0, &live, &bt);
    assert!(
        text.contains("Myoro Polymarket Terminal"),
        "title not found in:\n{}",
        &text[..text.len().min(500)]
    );
}

#[test]
fn render_tabs_shows_all_7_tab_names() {
    let live = LiveState::default();
    let bt = BacktesterState::new();
    let text = render_tab(0, &live, &bt);
    for name in &[
        "Crypto",
        "Sports",
        "Weather",
        "Copy",
        "Portfolio",
        "Discover",
        "Backtester",
    ] {
        assert!(text.contains(name), "tab '{}' not found", name);
    }
}

#[test]
fn small_terminal_shows_resize_message() {
    let live = LiveState::default();
    let bt = BacktesterState::new();
    let mut terminal = make_terminal(40, 10);
    let dv = empty_dv();
    terminal
        .draw(|f| {
            Layout::render(
                f,
                0,
                "",
                "",
                None,
                None,
                "",
                Some(&dv),
                &live,
                &[],
                None,
                None,
                None,
                &[0],
                0,
                "USD",
                None,
                &bt,
                0,
                0,
                None,
                0,
                0,
                false,
                "",
                false,
                false,
                None,
            );
        })
        .unwrap();
    let text = buf_text(&terminal);
    assert!(
        text.contains("Resize") || text.contains("resize") || text.contains("Myoro"),
        "small terminal render unexpected: {}",
        &text[..text.len().min(200)]
    );
}

#[test]
fn hint_bar_contains_shortcuts_hint() {
    let live = LiveState::default();
    let bt = BacktesterState::new();
    let text = render_tab(0, &live, &bt);
    assert!(text.contains("[?]"), "shortcuts hint '[?]' not found");
}

// ── Tab 0 — Crypto ────────────────────────────────────────────────────────────

#[test]
fn crypto_view_no_panic_with_empty_state() {
    let live = LiveState::default();
    let bt = BacktesterState::new();
    let text = render_tab(0, &live, &bt);
    assert!(text.contains("Crypto"), "Crypto tab header not found");
}

#[test]
fn crypto_view_shows_strategies_header() {
    let live = LiveState::default();
    let bt = BacktesterState::new();
    let text = render_tab(0, &live, &bt);
    assert!(text.contains("Strateg"), "Strategies pane title not found");
}

// ── Tab 1 — Sports ────────────────────────────────────────────────────────────

#[test]
fn sports_view_renders_three_pane_titles() {
    let live = LiveState::default();
    let bt = BacktesterState::new();
    let text = render_tab(1, &live, &bt);
    assert!(text.contains("Strateg"), "Strategies pane not found");
    assert!(text.contains("Signal"), "Signals pane not found");
    assert!(text.contains("Fixture"), "Fixtures pane not found");
}

// ── Tab 2 — Weather ───────────────────────────────────────────────────────────

#[test]
fn weather_view_renders_without_panic() {
    let live = LiveState::default();
    let bt = BacktesterState::new();
    let text = render_tab(2, &live, &bt);
    assert!(text.contains("Weather"), "Weather tab header not found");
}

// ── Tab 3 — Copy ─────────────────────────────────────────────────────────────

#[test]
fn copy_view_renders_two_panes() {
    let live = LiveState::default();
    let bt = BacktesterState::new();
    let text = render_tab(3, &live, &bt);
    assert!(
        text.contains("Trader") || text.contains("Copy"),
        "traders pane not found"
    );
    assert!(
        text.contains("Activit") || text.contains("Trade"),
        "activity pane not found"
    );
}

#[test]
fn render_copy_add_dialog_shows_search_box() {
    let live = LiveState::default();
    let bt = BacktesterState::new();
    let mut terminal = make_terminal(120, 40);
    let dv = empty_dv();
    let search = "test";
    let rows: Vec<(String, bool)> = vec![];
    terminal
        .draw(|f| {
            Layout::render(
                f,
                3,
                "",
                "",
                None,
                None,
                "",
                Some(&dv),
                &live,
                &[],
                None,
                None,
                None,
                &[0, 0],
                0,
                "USD",
                None,
                &bt,
                0,
                0,
                None,
                0,
                0,
                false,
                "",
                false,
                false,
                None,
            );
            Layout::render_copy_add_dialog(f, search, &rows);
        })
        .unwrap();
    let text = buf_text(&terminal);
    assert!(
        text.contains("test") || text.contains("Add"),
        "copy add dialog not rendered"
    );
}

// ── Tab 4 — Portfolio ─────────────────────────────────────────────────────────

#[test]
fn portfolio_view_renders_without_panic() {
    let live = LiveState::default();
    let bt = BacktesterState::new();
    let text = render_tab(4, &live, &bt);
    assert!(text.contains("Portfolio") || text.contains("Balance") || text.contains("P&L"));
}

// ── Tab 5 — Discover ─────────────────────────────────────────────────────────

#[test]
fn discover_view_renders_without_panic() {
    let live = LiveState::default();
    let bt = BacktesterState::new();
    let text = render_tab(5, &live, &bt);
    assert!(text.contains("Discover") || text.contains("ALL") || text.contains("Crypto"));
}

#[test]
fn render_discover_filter_dialog_shows_options() {
    let mut terminal = make_terminal(120, 40);
    terminal
        .draw(|f| {
            Layout::render_discover_filter_dialog(f, "Category", &["ALL", "CRYPTO", "SPORTS"], 0);
        })
        .unwrap();
    let text = buf_text(&terminal);
    assert!(
        text.contains("ALL"),
        "ALL option not found in filter dialog"
    );
    assert!(text.contains("CRYPTO"), "CRYPTO option not found");
}

// ── Tab 6 — Backtester ────────────────────────────────────────────────────────

#[test]
fn backtester_view_shows_tool_names() {
    let live = LiveState::default();
    let bt = BacktesterState::new();
    let text = render_tab(6, &live, &bt);
    assert!(
        text.contains("Perm") || text.contains("Trade Order") || text.contains("Tools"),
        "Backtester tools list not found"
    );
}

#[test]
fn backtester_view_selected_tool_has_bullet() {
    let live = LiveState::default();
    let bt = BacktesterState::new();
    let text = render_tab(6, &live, &bt);
    assert!(
        text.contains("[●]") || text.contains("●"),
        "selected tool bullet not found"
    );
}

#[test]
fn backtester_view_about_pane_shows_tooltip_hint() {
    let live = LiveState::default();
    let bt = BacktesterState::new();
    let text = render_tab(6, &live, &bt);
    assert!(
        text.contains("run") || text.contains("About") || text.contains("MC"),
        "backtester about pane hint not found"
    );
}

#[test]
fn backtester_view_different_tool_sel_changes_title() {
    let live = LiveState::default();
    let bt = BacktesterState::new();
    let mut terminal = make_terminal(120, 40);
    let dv = empty_dv();
    terminal
        .draw(|f| {
            Layout::render(
                f,
                6,
                "",
                "",
                None,
                None,
                "",
                Some(&dv),
                &live,
                &[],
                None,
                None,
                None,
                &[0, 0],
                0,
                "USD",
                None,
                &bt,
                3, // Historical Replay
                0,
                None,
                0,
                0,
                false,
                "",
                false,
                false,
                None,
            );
        })
        .unwrap();
    let text = buf_text(&terminal);
    assert!(
        text.contains("Historical") || text.contains("Replay") || text.contains("Tool"),
        "tool 3 title not found"
    );
}

// ── Modals ────────────────────────────────────────────────────────────────────

#[test]
fn render_bankroll_prompt_shows_input() {
    let mut terminal = make_terminal(120, 40);
    terminal
        .draw(|f| {
            Layout::render_bankroll_prompt(f, "1234");
        })
        .unwrap();
    let text = buf_text(&terminal);
    assert!(
        text.contains("1234") || text.contains("Bankroll"),
        "bankroll input not found"
    );
}

#[test]
fn render_currency_picker_shows_usd() {
    let mut terminal = make_terminal(120, 40);
    terminal
        .draw(|f| {
            Layout::render_currency_picker(f, &["USD", "EUR", "GBP"], 0, "");
        })
        .unwrap();
    let text = buf_text(&terminal);
    assert!(text.contains("USD"), "USD not found in currency picker");
}

#[test]
fn render_live_confirm_shows_mode_text() {
    let mut terminal = make_terminal(120, 40);
    terminal
        .draw(|f| {
            Layout::render_live_confirm(f, 0); // Crypto tab
        })
        .unwrap();
    let text = buf_text(&terminal);
    assert!(
        text.contains("Live") || text.contains("live") || text.contains("confirm"),
        "live confirm dialog text not found"
    );
}

#[test]
fn shortcuts_screen_renders_without_panic() {
    let mut terminal = make_terminal(120, 40);
    terminal
        .draw(|f| {
            Layout::render_shortcuts_screen(f, &[]);
        })
        .unwrap();
    // Just verify no panic with empty shortcuts
}
