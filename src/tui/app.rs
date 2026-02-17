use crate::config::ExecutionMode;
use crate::copy_trading::{Monitor, TraderList};
use crate::discover::DiscoverState;
use crate::live::LiveState;
use crate::tui::layout::{DiscoverView, Layout, ShortcutPair};
use crate::tui::theme::{
    self as theme_mod, add_custom_theme, current_theme_index, export_current_theme,
    import_theme, set_theme_index, theme_count, theme_name_at, ThemePalette, COLOR_PRESETS,
    THEME_CREATOR_ROLES,
};
use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

const NUM_TABS: usize = 5;

const DISCOVER_PAGE: usize = 25;

// Fixed widths for aligned columns. Selector uses 3 chars so columns don't shift.
const W_RANK: usize = 4;
const W_USER: usize = 12;
const W_VOL: usize = 12;
const W_PNL: usize = 10;
const W_ROI: usize = 6;
const W_TRADES: usize = 6;
const W_MAINLY: usize = 10;
const W_ADDR: usize = 12;

fn discover_view(
    entries: &[crate::discover::LeaderboardEntry],
    selected: Option<usize>,
    discover: &DiscoverState,
) -> DiscoverView {
    let loading = discover.is_fetching();
    let filters = format!(
        "Category\n  {}  [c] cycle\n\nPeriod\n  {}  [t] cycle\n\nOrder\n  {}  [o] cycle\n\n[r] Refresh",
        discover.category_label(),
        discover.time_period_label(),
        discover.order_by_label()
    );
    let (table, header, rows, scan_note) = if entries.is_empty() && !loading {
        ("No data. Press r to fetch.".to_string(), vec![], vec![], String::new())
    } else if entries.is_empty() {
        (String::new(), vec![], vec![], String::new())
    } else {
        let total = entries.len();
        let sel = selected.unwrap_or(0).min(total.saturating_sub(1));
        let start = (sel as i32 - 12).max(0).min((total - DISCOVER_PAGE).max(0) as i32) as usize;
        let end = (start + DISCOVER_PAGE).min(total);
        let header = vec![
            "".to_string(), "Rank".to_string(), "User".to_string(), "Vol".to_string(),
            "PnL".to_string(), "ROI%".to_string(), "Trades".to_string(), "Mainly".to_string(),
            "Address".to_string(),
        ];
        let mut row_data = Vec::new();
        for (idx, e) in entries[start..end].iter().enumerate() {
            let global_idx = start + idx;
            let selected_row = Some(global_idx) == selected;
            let roi = if e.vol > 0.0 { e.pnl / e.vol * 100.0 } else { 0.0 };
            let roi_positive = roi > 0.0;
            let stats = discover.get_stats(&e.proxy_wallet);
            let (trades, mainly) = stats
                .map(|s| (s.trade_count.to_string(), s.top_category.clone()))
                .unwrap_or_else(|| ("…".to_string(), "…".to_string()));
            let user = truncate_user(&e.user_name, W_USER).to_string();
            let mainly_short = if mainly.len() > W_MAINLY { format!("{}…", &mainly[..W_MAINLY - 1]) } else { mainly };
            let addr_short = e.proxy_wallet.get(..10).map(|s| format!("{}…", s)).unwrap_or_else(|| e.proxy_wallet.clone());
            let cells = vec![
                if selected_row { "►".to_string() } else { "".to_string() },
                format!("{}", e.rank),
                user,
                format!("{:>12.2}", e.vol),
                format!("{:>10.2}", e.pnl),
                format!("{:>5.1}%", roi),
                trades,
                mainly_short,
                addr_short,
            ];
            row_data.push((selected_row, roi_positive, cells));
        }
        let table_str = format!(
            "Profiles: {} (showing {}-{})   ↑↓ / jk  scroll   a / Enter  add to copy",
            total, start + 1, end
        );
        let note = "Background scan: Trades + Mainly fill in as profiles are fetched.";
        (table_str, header, row_data, note.to_string())
    };
    DiscoverView {
        filters,
        table,
        leaderboard_header: header,
        leaderboard_rows: rows,
        scan_note,
        loading,
    }
}

fn truncate_user(s: &str, max: usize) -> &str {
    if s.len() <= max {
        s
    } else {
        &s[..max]
    }
}

pub fn run() -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let _ = dotenvy::dotenv();
    let config = crate::config::load().unwrap_or_default();
    let live_state = Arc::new(LiveState::default());
    if let Some(b) = config.paper_bankroll {
        live_state.set_bankroll(Some(b));
    }
    let trader_list = Arc::new(TraderList::new());
    let copy_running = Arc::new(AtomicBool::new(false));
    let monitor = Arc::new(Monitor::new(
        Arc::clone(&trader_list),
        Some(Arc::clone(&live_state)),
        Arc::clone(&copy_running),
    ));

    let poll_ms = Monitor::poll_ms();
    let monitor_clone = Arc::clone(&monitor);
    std::thread::spawn(move || loop {
        monitor_clone.poll_once();
        std::thread::sleep(Duration::from_millis(poll_ms));
    });

    let live_clone = Arc::clone(&live_state);
    std::thread::spawn(move || loop {
        live_clone.fetch_all();
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
    theme_mod::init_themes();
    let res = run_loop(
        &mut terminal,
        &monitor,
        &live_state,
        &trader_list,
        &discover_state,
        &copy_running,
    );

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    res
}

fn run_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    monitor: &Monitor,
    live: &LiveState,
    trader_list: &Arc<TraderList>,
    discover: &Arc<DiscoverState>,
    copy_running: &AtomicBool,
) -> Result<()> {
    let mut selected_tab = 0usize;
    let mut copy_selected: Option<usize> = None;
    let mut copy_input = String::new();
    let mut discover_selected: Option<usize> = None;
    let mut tab_execution_mode: [ExecutionMode; 5] = [ExecutionMode::Paper; 5];
    let mut show_theme_overlay = false;
    let mut theme_overlay_selection = 0usize;
    let mut theme_in_creator = false;
    let mut theme_creator_role = 0usize;
    let mut theme_creator_color_idx = 0usize;
    let mut theme_editor_palette: Option<ThemePalette> = None;
    let mut live_confirm_tab: Option<usize> = None;
    let mut bankroll_input: Option<String> = None;

    loop {
        let list = monitor.trader_list();
        let n_addr = list.len();
        if selected_tab == 3 {
            copy_selected = copy_selected.map(|i| i.min(n_addr.saturating_sub(1)));
        }
        let discover_entries = discover.get_entries();
        if selected_tab == 4 {
            discover_selected = discover_selected.map(|i| i.min(discover_entries.len().saturating_sub(1)));
        }

        let copy_text = monitor.copy_tab_display(copy_selected, &copy_input);
        let discover_view = discover_view(&discover_entries, discover_selected, discover);
        type ShortcutCategory = (String, Vec<ShortcutPair>);
        let base_nav: ShortcutCategory = (
            "Navigation".into(),
            vec![
                ("E".into(), "Next tab".into()),
                ("Q".into(), "Prev tab".into()),
                ("1-5".into(), "Jump to tab".into()),
                ("↑/↓/j/k".into(), "Move".into()),
            ],
        );
        let base_global: ShortcutCategory = (
            "Global".into(),
            vec![
                ("Esc".into(), "Quit".into()),
                ("T".into(), "Theming".into()),
                ("b".into(), "Set bankroll".into()),
            ],
        );
        let base_mode: ShortcutCategory = ("Mode".into(), vec![("m".into(), "Paper/Live".into())]);
        let shortcuts: Vec<ShortcutCategory> = match selected_tab {
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
                        ("↑/↓/j/k".into(), "Select".into()),
                    ],
                ),
            ],
            4 => vec![
                base_nav,
                base_global,
                base_mode,
                (
                    "Discover".into(),
                    vec![
                        ("r".into(), "Refresh".into()),
                        ("c".into(), "Category".into()),
                        ("t".into(), "Period".into()),
                        ("o".into(), "Order".into()),
                        ("a / Enter".into(), "Add to copy".into()),
                        ("↑/↓/j/k".into(), "Scroll".into()),
                    ],
                ),
            ],
            _ => vec![base_nav, base_global, base_mode],
        };
        let dv = (selected_tab == 4).then(|| discover_view.clone());
        let mode_str = (selected_tab < 4).then(|| match tab_execution_mode[selected_tab] {
            ExecutionMode::Live => "Live",
            ExecutionMode::Paper => "Paper",
        });
        terminal.draw(|f| {
            if show_theme_overlay {
                let n = theme_count();
                let theme_names: Vec<String> = (0..n).map(theme_name_at).collect();
                let cur = current_theme_index();
                let sel = if theme_in_creator {
                    theme_overlay_selection
                } else {
                    theme_overlay_selection.min(n + 2)
                };
                Layout::render_theme_screen(
                    f,
                    sel,
                    &theme_names,
                    cur,
                    theme_in_creator,
                    theme_creator_role,
                    theme_creator_color_idx,
                    theme_editor_palette.as_ref(),
                );
            } else {
                Layout::render(
                    f,
                    selected_tab,
                    &copy_text,
                    "",
                    dv.as_ref(),
                    live,
                    &shortcuts,
                    mode_str.as_deref(),
                );
                if let Some(tab) = live_confirm_tab {
                    Layout::render_live_confirm(f, tab);
                }
                if let Some(ref s) = bankroll_input {
                    Layout::render_bankroll_prompt(f, s);
                }
            }
        })?;

        if event::poll(std::time::Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if key.kind != KeyEventKind::Press {
                    continue;
                }
                if live_confirm_tab.is_some() {
                    match key.code {
                        KeyCode::Char('y') => {
                            if let Some(tab) = live_confirm_tab {
                                tab_execution_mode[tab] = ExecutionMode::Live;
                                live_confirm_tab = None;
                            }
                        }
                        KeyCode::Char('n') | KeyCode::Esc => live_confirm_tab = None,
                        _ => {}
                    }
                    continue;
                }
                if let Some(ref mut s) = bankroll_input {
                    match key.code {
                        KeyCode::Esc => bankroll_input = None,
                        KeyCode::Enter => {
                            if let Ok(v) = s.trim().parse::<f64>() {
                                if v >= 0.0 {
                                    live.set_bankroll(Some(v));
                                }
                            }
                            bankroll_input = None;
                        }
                        KeyCode::Backspace => { s.pop(); }
                        KeyCode::Char(c) if c.is_ascii_digit() || c == '.' => {
                            if c == '.' && s.contains('.') { } else { s.push(c); }
                        }
                        _ => {}
                    }
                    continue;
                }
                if show_theme_overlay {
                    let n = theme_count();
                    let total_items = n + 3; // themes + Export + Import + Creator
                    if theme_in_creator {
                        match key.code {
                            KeyCode::Esc => {
                                theme_in_creator = false;
                                theme_editor_palette = None;
                            }
                            KeyCode::Char('s') => {
                                if let Some(mut p) = theme_editor_palette.take() {
                                    p.name = format!("Custom {}", n);
                                    let idx = add_custom_theme(p);
                                    set_theme_index(idx);
                                    theme_in_creator = false;
                                }
                            }
                            KeyCode::Char('j') | KeyCode::Down => {
                                theme_creator_role = (theme_creator_role + 1).min(THEME_CREATOR_ROLES.len().saturating_sub(1));
                            }
                            KeyCode::Char('k') | KeyCode::Up => {
                                theme_creator_role = theme_creator_role.saturating_sub(1);
                            }
                            KeyCode::Char('l') | KeyCode::Right => {
                                theme_creator_color_idx = (theme_creator_color_idx + 1).min(COLOR_PRESETS.len().saturating_sub(1));
                                if let Some(ref mut p) = theme_editor_palette {
                                    p.set_role_color(theme_creator_role, COLOR_PRESETS[theme_creator_color_idx]);
                                }
                            }
                            KeyCode::Char('h') | KeyCode::Left => {
                                theme_creator_color_idx = theme_creator_color_idx.saturating_sub(1);
                                if let Some(ref mut p) = theme_editor_palette {
                                    p.set_role_color(theme_creator_role, COLOR_PRESETS[theme_creator_color_idx]);
                                }
                            }
                            _ => {}
                        }
                    } else {
                        match key.code {
                            KeyCode::Char('T') | KeyCode::F(10) | KeyCode::Esc => {
                                show_theme_overlay = false;
                            }
                            KeyCode::Char('j') | KeyCode::Down => {
                                theme_overlay_selection = (theme_overlay_selection + 1).min(total_items.saturating_sub(1));
                            }
                            KeyCode::Char('k') | KeyCode::Up => {
                                theme_overlay_selection = theme_overlay_selection.saturating_sub(1);
                            }
                            KeyCode::Enter => {
                                if theme_overlay_selection < n {
                                    set_theme_index(theme_overlay_selection);
                                } else if theme_overlay_selection == n + 2 {
                                    theme_in_creator = true;
                                    theme_editor_palette = Some(theme_mod::current_palette());
                                    theme_creator_role = 0;
                                    theme_creator_color_idx = 0;
                                }
                            }
                            KeyCode::Char('e') => {
                                let path = std::path::Path::new("theme_export.toml");
                                let _ = export_current_theme(path);
                            }
                            KeyCode::Char('i') => {
                                let path = std::path::Path::new("theme_import.toml");
                                if let Ok(idx) = import_theme(path) {
                                    set_theme_index(idx);
                                }
                            }
                            KeyCode::Char('c') => {
                                theme_in_creator = true;
                                theme_editor_palette = Some(theme_mod::current_palette());
                                theme_creator_role = 0;
                                theme_creator_color_idx = 0;
                            }
                            _ => {}
                        }
                    }
                    continue;
                }
                if !copy_input.is_empty() {
                    match key.code {
                        KeyCode::Char(c) => copy_input.push(c),
                        KeyCode::Backspace => {
                            copy_input.pop();
                        }
                        KeyCode::Enter => {
                            if trader_list.add(copy_input.clone()) {
                                copy_input.clear();
                            }
                        }
                        KeyCode::Esc => copy_input.clear(),
                        _ => {}
                    }
                    continue;
                }
                match key.code {
                    KeyCode::Char('T') | KeyCode::F(10) => show_theme_overlay = true,
                    KeyCode::Char('b') => bankroll_input = Some(String::new()),
                    KeyCode::Esc => break,
                    KeyCode::Char('q') => {
                        selected_tab = if selected_tab == 0 {
                            NUM_TABS - 1
                        } else {
                            selected_tab - 1
                        };
                    }
                    KeyCode::Char('e') => {
                        selected_tab = if selected_tab + 1 >= NUM_TABS {
                            0
                        } else {
                            selected_tab + 1
                        };
                    }
                    KeyCode::Left => selected_tab = selected_tab.saturating_sub(1),
                    KeyCode::Right => selected_tab = (selected_tab + 1).min(NUM_TABS - 1),
                    KeyCode::Tab => {
                        selected_tab = if selected_tab + 1 >= NUM_TABS {
                            0
                        } else {
                            selected_tab + 1
                        };
                    }
                    KeyCode::BackTab => {
                        selected_tab = if selected_tab == 0 {
                            NUM_TABS - 1
                        } else {
                            selected_tab - 1
                        };
                    }
                    KeyCode::Char('1') => selected_tab = 0,
                    KeyCode::Char('2') => selected_tab = 1,
                    KeyCode::Char('3') => selected_tab = 2,
                    KeyCode::Char('4') => selected_tab = 3,
                    KeyCode::Char('5') => selected_tab = 4,
                    KeyCode::Char('m') => {
                        let idx = selected_tab.min(4);
                        if tab_execution_mode[idx] == ExecutionMode::Paper {
                            live_confirm_tab = Some(idx);
                        } else {
                            tab_execution_mode[idx] = ExecutionMode::Paper;
                        }
                    }
                    _ => {
                        if selected_tab == 3 {
                            match key.code {
                                KeyCode::Char('s') => {
                                    let v = copy_running.load(Ordering::SeqCst);
                                    copy_running.store(!v, Ordering::SeqCst);
                                }
                                KeyCode::Char('a') => copy_input = String::new(),
                                KeyCode::Char('d') => {
                                    if let Some(i) = copy_selected {
                                        trader_list.remove_at(i);
                                        copy_selected = if n_addr <= 1 {
                                            None
                                        } else {
                                            Some(i.saturating_sub(1).min(n_addr.saturating_sub(2)))
                                        };
                                    }
                                }
                                KeyCode::Up | KeyCode::Char('k') => {
                                    copy_selected = match copy_selected {
                                        None => if n_addr > 0 { Some(n_addr - 1) } else { None },
                                        Some(0) => Some(0),
                                        Some(i) => Some(i - 1),
                                    };
                                }
                                KeyCode::Down | KeyCode::Char('j') => {
                                    copy_selected = match copy_selected {
                                        None => if n_addr > 0 { Some(0) } else { None },
                                        Some(i) if i + 1 >= n_addr => Some(i),
                                        Some(i) => Some(i + 1),
                                    };
                                }
                                _ => {}
                            }
                        } else if selected_tab == 4 {
                            let n_d = discover_entries.len();
                            match key.code {
                                KeyCode::Char('r') => {
                                    let d = Arc::clone(discover);
                                    std::thread::spawn(move || d.fetch());
                                }
                                KeyCode::Char('c') => {
                                    discover.cycle_category();
                                    let d = Arc::clone(discover);
                                    std::thread::spawn(move || d.fetch());
                                }
                                KeyCode::Char('t') => {
                                    discover.cycle_time_period();
                                    let d = Arc::clone(discover);
                                    std::thread::spawn(move || d.fetch());
                                }
                                KeyCode::Char('o') => {
                                    discover.cycle_order_by();
                                    let d = Arc::clone(discover);
                                    std::thread::spawn(move || d.fetch());
                                }
                                KeyCode::Char('a') | KeyCode::Enter => {
                                    if let Some(i) = discover_selected {
                                        if let Some(e) = discover_entries.get(i) {
                                            if trader_list.add(e.proxy_wallet.clone()) {
                                                live.push_copy_log(crate::live::LogLevel::Success, format!("Added {} to copy list", e.proxy_wallet));
                                            }
                                        }
                                    }
                                }
                                KeyCode::Up | KeyCode::Char('k') => {
                                    discover_selected = match discover_selected {
                                        None => if n_d > 0 { Some(n_d - 1) } else { None },
                                        Some(0) => Some(0),
                                        Some(i) => Some(i - 1),
                                    };
                                }
                                KeyCode::Down | KeyCode::Char('j') => {
                                    discover_selected = match discover_selected {
                                        None => if n_d > 0 { Some(0) } else { None },
                                        Some(i) if i + 1 >= n_d => Some(i),
                                        Some(i) => Some(i + 1),
                                    };
                                }
                                _ => {}
                            }
                        }
                    }
                }
            }
        }
    }
    Ok(())
}
