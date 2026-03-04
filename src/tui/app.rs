// app.rs: TUI terminal setup, thread spawning, and helper builders.
use crate::backtester::BacktesterState;
use crate::copy_trading::{Monitor, TraderList};
use crate::discover::DiscoverState;
use crate::live::LiveState;
use crate::tui::events::SportsUiState;
use crate::tui::layout::{DiscoverView, ShortcutPair, SportsView};
use crate::tui::runner::run_loop;
use crate::tui::theme as theme_mod;
use anyhow::Result;
use crossterm::{
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::time::Duration;

pub fn run(initial_config: crate::config::Config) -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let config = Arc::new(std::sync::RwLock::new(initial_config));
    let live_state = Arc::new(LiveState::default());
    if let Ok(c) = config.read() {
        if let Some(b) = c.paper_bankroll {
            live_state.set_bankroll(Some(b));
        }
    }
    let trader_list = Arc::new(TraderList::new(Arc::clone(&config)));
    let copy_running = Arc::new(AtomicBool::new(false));
    let monitor = Arc::new(Monitor::new(
        Arc::clone(&trader_list),
        Some(Arc::clone(&live_state)),
        Arc::clone(&copy_running),
    ));

    let config_poll = Arc::clone(&config);
    let monitor_clone = Arc::clone(&monitor);
    std::thread::spawn(move || loop {
        monitor_clone.poll_once();
        let ms = config_poll
            .read()
            .map(|c| Monitor::poll_ms_from_config(&c))
            .unwrap_or(250);
        std::thread::sleep(Duration::from_millis(ms));
    });

    let live_clone = Arc::clone(&live_state);
    let config_mm = Arc::clone(&config);
    std::thread::spawn(move || loop {
        live_clone.fetch_all();
        if let Ok(cfg) = config_mm.read() {
            live_clone.run_mm(&cfg);
        }
        std::thread::sleep(Duration::from_secs(8));
    });

    let discover_state = Arc::new(DiscoverState::new());
    {
        let d = Arc::clone(&discover_state);
        std::thread::spawn(move || d.fetch());
    }
    let discover_clone = Arc::clone(&discover_state);
    std::thread::spawn(move || loop {
        discover_clone.scan_next();
        std::thread::sleep(Duration::from_millis(500));
    });

    let backtester_state = BacktesterState::new();
    {
        let bt_clone = Arc::clone(&backtester_state);
        std::thread::spawn(move || bt_clone.run_all("data/paper_trades.jsonl"));
    }

    theme_mod::init_themes();
    let res = run_loop(
        &mut terminal,
        &monitor,
        &live_state,
        &trader_list,
        &discover_state,
        &copy_running,
        &config,
        &backtester_state,
    );

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    res
}

// ── Helpers used by runner.rs ─────────────────────────────────────────────────

pub(crate) fn build_shortcuts(selected_tab: usize) -> Vec<(String, Vec<ShortcutPair>)> {
    let base_nav = (
        "Navigation".into(),
        vec![
            ("E / Q".into(), "Next/prev tab".into()),
            ("1-7".into(), "Jump to tab".into()),
            ("Tab".into(), "Focus section".into()),
            ("↑/↓ j/k".into(), "Scroll line".into()),
            ("h/l ←/→".into(), "Scroll page".into()),
        ],
    );
    let base_global = (
        "Global".into(),
        vec![
            ("Esc".into(), "Quit".into()),
            ("T".into(), "Theming".into()),
            ("b".into(), "Set bankroll".into()),
            ("C".into(), "P&L currency".into()),
        ],
    );
    let base_mode = ("Mode".into(), vec![("m".into(), "Paper/Live".into())]);
    match selected_tab {
        3 => vec![
            base_nav,
            base_global,
            base_mode,
            (
                "Copy".into(),
                vec![
                    ("s".into(), "Start/stop trading".into()),
                    ("a".into(), "Add trader".into()),
                    ("d".into(), "Remove selected".into()),
                ],
            ),
        ],
        5 => vec![
            base_nav,
            base_global,
            base_mode,
            (
                "Discover".into(),
                vec![
                    ("r".into(), "Refresh".into()),
                    ("s".into(), "Screener".into()),
                    ("c".into(), "Category".into()),
                    ("t".into(), "Period".into()),
                    ("o".into(), "Order".into()),
                    ("a / Enter".into(), "Add to copy".into()),
                ],
            ),
        ],
        6 => vec![
            base_nav,
            base_global,
            (
                "Backtester".into(),
                vec![
                    ("Tab".into(), "Switch pane".into()),
                    ("↑/↓".into(), "Select item".into()),
                    ("←/→".into(), "Adjust param".into()),
                    ("r".into(), "Run analysis".into()),
                    ("Enter".into(), "Edit param".into()),
                    ("Esc".into(), "Cancel edit".into()),
                ],
            ),
        ],
        _ => vec![base_nav, base_global, base_mode],
    }
}

pub(crate) fn discover_view_for_render(
    entries: &[crate::discover::LeaderboardEntry],
    selected: Option<usize>,
    discover: &DiscoverState,
    copy_addresses: &[String],
    max_rows: usize,
) -> DiscoverView {
    crate::tui::views::discover::build_view(entries, selected, discover, copy_addresses, max_rows)
}

pub(crate) fn sports_view_for_render(
    live: &LiveState,
    ui: &SportsUiState,
    scroll: usize,
) -> SportsView {
    let _ = scroll;
    crate::tui::views::sports::build_view(
        live,
        ui.pane,
        ui.strategy_sel,
        ui.signal_sel,
        ui.fixture_sel,
        ui.league_filter.as_deref(),
        ui.team_filter.as_deref(),
        ui.show_league_picker,
        ui.show_team_picker,
        ui.league_picker_sel,
        ui.team_picker_sel,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_shortcuts_global_has_base_sections() {
        let s = build_shortcuts(0);
        assert!(!s.is_empty());
        assert_eq!(s[0].0, "Navigation");
    }

    #[test]
    fn build_shortcuts_copy_tab_has_copy_section() {
        let s = build_shortcuts(3);
        let has_copy = s.iter().any(|(name, _)| name == "Copy");
        assert!(has_copy);
    }

    #[test]
    fn build_shortcuts_discover_tab_has_discover_section() {
        let s = build_shortcuts(5);
        let has_discover = s.iter().any(|(name, _)| name == "Discover");
        assert!(has_discover);
    }
}
