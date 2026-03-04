// runner.rs: main TUI event loop — thin coordinator over RouterState + handle_key.
use crate::copy_trading::{Monitor, TraderList};
use crate::discover::DiscoverState;
use crate::live::LiveState;
use crate::tui::app::{build_shortcuts, discover_view_for_render, sports_view_for_render};
use crate::tui::layout::Layout;
use crate::tui::router::{AppCtx, RouterEffect, RouterState};
use crate::tui::theme::{self as theme_mod, current_theme_index, theme_count};
use anyhow::Result;
use crossterm::event::{self, Event, KeyEventKind};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

const CURRENCIES: &[&str] = &["USD", "EUR", "GBP", "BTC", "ETH"];

#[allow(clippy::too_many_arguments)]
pub fn run_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    monitor: &Monitor,
    live: &LiveState,
    trader_list: &Arc<TraderList>,
    discover: &Arc<DiscoverState>,
    copy_running: &AtomicBool,
    config: &Arc<std::sync::RwLock<crate::config::Config>>,
    backtester: &Arc<crate::backtester::BacktesterState>,
) -> Result<()> {
    let mut rs = RouterState::default();

    loop {
        // ── Clamp selection indices ───────────────────────────────────────────
        let list = monitor.trader_list();
        let n_addr = list.len();
        if rs.selected_tab == 3 {
            rs.copy_selected = if n_addr == 0 {
                None
            } else if rs.focused_section == 0 {
                Some(rs.scroll_offsets[3][0].min(n_addr.saturating_sub(1)))
            } else {
                rs.copy_selected.map(|i| i.min(n_addr.saturating_sub(1)))
            };
        }
        let discover_entries = discover.get_entries();
        if rs.selected_tab == 5 {
            let n_d = discover_entries.len();
            rs.discover_selected = if n_d == 0 {
                None
            } else {
                Some(rs.scroll_offsets[5][0].min(n_d.saturating_sub(1)))
            };
        }

        // ── Build render helpers ──────────────────────────────────────────────
        let copy_addresses = trader_list.get_addresses();
        let copy_list_text = crate::tui::views::copy::build_list_content(
            copy_addresses.clone(),
            rs.copy_selected,
            &discover_entries,
        );
        let copy_trades_text = crate::tui::views::copy::build_trades_content(monitor);
        let (copy_status_line, copy_last_line) = {
            let running = copy_running.load(Ordering::SeqCst);
            let (auto_exec, sizing) = config
                .read()
                .map(|c| {
                    let s = match c.copy_sizing {
                        crate::config::CopySizing::Proportional => "proportional",
                        crate::config::CopySizing::Fixed => "fixed",
                    };
                    (c.copy_auto_execute, s.to_string())
                })
                .unwrap_or((false, "proportional".to_string()));
            let bankroll = live
                .global_stats
                .read()
                .ok()
                .and_then(|s| s.bankroll)
                .map(|b| format!("${:.2}", b))
                .unwrap_or_else(|| "—".to_string());
            let status_line =
                crate::tui::views::copy::build_status_line(running, auto_exec, &sizing, &bankroll);
            let last_line = live
                .get_copy_logs()
                .into_iter()
                .rev()
                .find_map(|(_, msg)| msg.contains("Paper copy:").then_some(msg))
                .unwrap_or_else(|| "—".to_string());
            (status_line, last_line)
        };

        let shortcuts = build_shortcuts(rs.selected_tab);
        let mode_str =
            (rs.selected_tab < 4).then(|| match rs.tab_execution_mode[rs.selected_tab] {
                crate::config::ExecutionMode::Live => "Live",
                crate::config::ExecutionMode::Paper => "Paper",
            });
        let copy_status = (rs.selected_tab == 3).then(|| {
            if copy_running.load(Ordering::SeqCst) {
                "Running"
            } else {
                "Stopped"
            }
        });
        let copy_status_style = (rs.selected_tab == 3).then(|| {
            if copy_running.load(Ordering::SeqCst) {
                theme_mod::Theme::success()
            } else {
                theme_mod::Theme::danger()
            }
        });
        let pnl_currency = config
            .read()
            .map(|c| c.pnl_currency.clone())
            .unwrap_or_else(|_| "USD".to_string());

        // ── Render ────────────────────────────────────────────────────────────
        terminal.draw(|f| {
            if rs.show_shortcuts_screen {
                Layout::render_shortcuts_screen(f, &shortcuts);
            } else if rs.show_theme_overlay {
                let n = theme_count();
                let theme_names: Vec<String> =
                    (0..n).map(crate::tui::theme::theme_name_at).collect();
                let cur = current_theme_index();
                let sel = if rs.theme_in_creator {
                    rs.theme_overlay_sel
                } else {
                    rs.theme_overlay_sel.min(n + 2)
                };
                Layout::render_theme_screen(
                    f,
                    sel,
                    &theme_names,
                    cur,
                    rs.theme_in_creator,
                    rs.theme_creator_role,
                    rs.theme_creator_color_idx,
                    rs.theme_editor_palette.as_ref(),
                );
            } else {
                let area = f.area();
                let table_height = (area.height as usize).saturating_sub(17).clamp(1, 500);
                let dv = discover_view_for_render(
                    &discover_entries,
                    rs.discover_selected,
                    discover,
                    &copy_addresses,
                    table_height,
                );
                let sv = (rs.selected_tab == 1)
                    .then(|| sports_view_for_render(live, &rs.sports_ui, rs.scroll_offsets[1][0]));
                #[allow(clippy::needless_option_as_deref)]
                Layout::render(
                    f,
                    rs.selected_tab,
                    &copy_list_text,
                    &copy_trades_text,
                    Some(copy_status_line.as_str()),
                    Some(copy_last_line.as_str()),
                    "",
                    Some(&dv),
                    live,
                    &shortcuts,
                    mode_str.as_deref(),
                    copy_status.as_deref(),
                    copy_status_style,
                    &rs.scroll_offsets[rs.selected_tab],
                    rs.focused_section,
                    &pnl_currency,
                    sv.as_ref(),
                    backtester,
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
                if let Some(dialog) = rs.discover_filter_dialog {
                    let (title, options, sel) = match dialog {
                        crate::tui::events::discover::DiscoverFilterDialog::Category(i) => (
                            "Category",
                            &[
                                "ALL",
                                "CRYPTO",
                                "SPORTS",
                                "POLITICS",
                                "CULTURE",
                                "WEATHER",
                                "ECONOMICS",
                                "TECH",
                                "FINANCE",
                            ][..],
                            i,
                        ),
                        crate::tui::events::discover::DiscoverFilterDialog::Period(i) => {
                            ("Period", &["ALL", "DAY", "WEEK", "MONTH"][..], i)
                        }
                        crate::tui::events::discover::DiscoverFilterDialog::Order(i) => {
                            ("Order", &["P&L", "VOL"][..], i)
                        }
                    };
                    Layout::render_discover_filter_dialog(f, title, options, sel);
                }
                if let Some(ref s) = rs.bankroll_input {
                    Layout::render_bankroll_prompt(f, s);
                }
                if rs.show_currency_picker {
                    let filtered: Vec<&str> = CURRENCIES
                        .iter()
                        .filter(|c| {
                            rs.currency_filter.is_empty()
                                || c.to_lowercase()
                                    .contains(&rs.currency_filter.to_lowercase())
                        })
                        .copied()
                        .collect();
                    Layout::render_currency_picker(
                        f,
                        &filtered,
                        rs.currency_selected,
                        &rs.currency_filter,
                    );
                }
                if let Some((ref search, sel_idx)) = rs.copy_add_dialog {
                    let opts =
                        crate::tui::events::copy::build_add_options(search, &discover_entries);
                    let si = sel_idx.min(opts.len().saturating_sub(1));
                    let rows: Vec<(String, bool)> = opts
                        .iter()
                        .enumerate()
                        .map(|(i, o)| (o.display_line(), i == si))
                        .collect();
                    Layout::render_copy_add_dialog(f, search, &rows);
                }
            }
        })?;

        // ── Poll + dispatch ───────────────────────────────────────────────────
        if !event::poll(Duration::from_millis(100))? {
            continue;
        }
        let Event::Key(key) = event::read()? else {
            continue;
        };
        if key.kind != KeyEventKind::Press {
            continue;
        }

        let ctx = AppCtx {
            live,
            trader_list,
            discover,
            copy_running,
            config,
            backtester,
        };
        if matches!(
            crate::tui::router::handle_key(&mut rs, key.code, &ctx),
            RouterEffect::Quit
        ) {
            break;
        }
    }
    Ok(())
}
