// NOTE: Exceeds 300-line limit — layout dispatcher + re-exports + dialog delegators; the delegation methods must stay together to maintain a single call-site for runner.rs. See docs/ai-rules/file-size.md
// layout.rs: shared view types + thin Layout dispatcher.
use ratatui::{
    layout::{Constraint, Direction, Layout as RLayout, Rect},
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};

use super::theme::Theme;
use crate::live::LiveState;

// Re-export view types so existing callers (`use crate::tui::layout::*`) still work.
pub use crate::tui::views::types::{
    DiscoverView, FixtureRow, ShortcutPair, SignalRow, SportsView, StrategyRow, TABS,
};

const PADDING_H: u16 = 0;
const PADDING_V: u16 = 0;
const TITLE_MARGIN_EXTRA: u16 = 0;
const MIN_TERMINAL_WIDTH: u16 = 60;
const MIN_TERMINAL_HEIGHT: u16 = 24;

pub struct Layout;

// ── Helpers ───────────────────────────────────────────────────────────────────

fn inner_area(area: Rect) -> Rect {
    Rect {
        x: area.x + PADDING_H,
        y: area.y + PADDING_V,
        width: area.width.saturating_sub(2 * PADDING_H),
        height: area.height.saturating_sub(2 * PADDING_V),
    }
}

fn dimensions_too_small_messages() -> &'static [&'static str] {
    &[
        "Resize me, I'm not a fan of tight spaces.",
        "Your terminal is shy. Give it some room.",
        "Small screen, big dreams. Resize to continue.",
        "This terminal has seen smaller days. Resize up.",
        "Need more pixels. Enlarge the window.",
        "Like a bonsai, but for terminals. Resize to grow.",
        "Dimensions: smol. Required: less smol.",
        "The UI needs legroom. Resize the window.",
        "Too cozy. Expand the terminal.",
        "Increase dimensions or decrease expectations.",
    ]
}

// ── Main dispatcher ───────────────────────────────────────────────────────────

impl Layout {
    #[allow(clippy::too_many_arguments)]
    pub fn render(
        f: &mut Frame,
        selected_tab: usize,
        copy_list_content: &str,
        copy_trades_content: &str,
        copy_status_line: Option<&str>,
        copy_last_line: Option<&str>,
        _discover_content: &str,
        discover_view: Option<&DiscoverView>,
        live: &LiveState,
        _shortcuts: &[(String, Vec<ShortcutPair>)],
        tab_mode: Option<&str>,
        copy_status: Option<&str>,
        copy_status_style: Option<Style>,
        section_scroll_offsets: &[usize],
        focused_section: usize,
        pnl_currency: &str,
        sports_view: Option<&SportsView>,
        backtester: &std::sync::Arc<crate::backtester::BacktesterState>,
        backtest_tool_sel: usize,
        backtest_strategy_sel: usize,
        backtest_selected_strategy: Option<usize>,
        backtest_data_sel: usize,
        backtest_param_sel: usize,
        backtest_param_editing: bool,
        backtest_param_input: &str,
        backtest_show_graph: bool,
        backtest_show_help: bool,
        backtest_data_dialog: Option<&crate::tui::router::DataConfigDialog>,
    ) {
        let area = f.area();
        f.render_widget(
            Block::default().style(Style::default().bg(Theme::BG())),
            area,
        );

        if area.width < MIN_TERMINAL_WIDTH || area.height < MIN_TERMINAL_HEIGHT {
            let messages = dimensions_too_small_messages();
            let idx = (area.width as usize + area.height as usize) % messages.len();
            let inner = inner_area(area);
            let para = Paragraph::new(vec![
                Line::from(Span::styled(
                    "Myoro Polymarket Terminal",
                    Theme::block_title(),
                )),
                Line::from(Span::styled(messages[idx], Theme::body())),
            ])
            .alignment(ratatui::layout::Alignment::Center);
            f.render_widget(para, inner);
            return;
        }

        let inner = inner_area(area);
        if inner.width == 0 || inner.height == 0 {
            return;
        }
        let tab_index = selected_tab.min(TABS.len().saturating_sub(1));
        let is_main_tab = tab_index <= 4;
        // Backtester gets a slimmer layout without the metrics row
        let is_backtester = tab_index == 6;

        // chunks[0]=title, chunks[1]=metrics(if applicable), chunks[last-1]=tabs, etc.
        // Layout A (main tabs 0-4):  title | metrics | tabs | content | trades | hint
        // Layout B (discover tab 5): title | metrics | tabs | content | hint
        // Layout C (backtester 6):   title | tabs | content | hint
        let (chunks, tabs_idx, content_idx, trades_idx_opt, hint_idx) = if is_main_tab {
            let c = RLayout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(3),
                    Constraint::Length(3),
                    Constraint::Length(3),
                    Constraint::Min(4),
                    Constraint::Length(2),
                    Constraint::Length(1),
                ])
                .split(inner);
            (c, 2usize, 3usize, Some(4usize), 5usize)
        } else if is_backtester {
            let c = RLayout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(3),
                    Constraint::Length(3),
                    Constraint::Min(4),
                    Constraint::Length(1),
                ])
                .split(inner);
            (c, 1usize, 2usize, None, 3usize)
        } else {
            let c = RLayout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(3),
                    Constraint::Length(3),
                    Constraint::Length(3),
                    Constraint::Min(4),
                    Constraint::Length(1),
                ])
                .split(inner);
            (c, 2usize, 3usize, None, 4usize)
        };

        let title_area = Rect {
            x: chunks[0].x + TITLE_MARGIN_EXTRA,
            y: chunks[0].y,
            width: chunks[0].width.saturating_sub(2 * TITLE_MARGIN_EXTRA),
            height: chunks[0].height,
        };
        crate::tui::views::header::render_title(f, title_area);
        if !is_backtester {
            crate::tui::views::header::render_metrics(
                f,
                chunks[1],
                live,
                pnl_currency,
                tab_mode,
                copy_status,
                copy_status_style,
            );
        }
        crate::tui::views::header::render_tabs(f, chunks[tabs_idx], tab_index);

        let main_rect = chunks[content_idx];
        let trades_rect = trades_idx_opt.map(|i| chunks[i]).unwrap_or_default();
        let indicator_idx = hint_idx;

        let scroll = section_scroll_offsets.first().copied().unwrap_or(0);
        match tab_index {
            0 => crate::tui::views::crypto::render(f, main_rect, live, section_scroll_offsets),
            1 => {
                if let Some(sv) = sports_view {
                    crate::tui::views::sports::render(f, main_rect, sv, scroll);
                } else {
                    f.render_widget(
                        Paragraph::new("⏳ Loading sports data…")
                            .style(Theme::body())
                            .wrap(Wrap { trim: true })
                            .block(crate::tui::views::header::bordered_block("Sports")),
                        main_rect,
                    );
                }
            }
            2 => crate::tui::views::weather::render(f, main_rect, live, scroll),
            3 => {
                let s0 = section_scroll_offsets.first().copied().unwrap_or(0) as u16;
                let s1 = section_scroll_offsets.get(1).copied().unwrap_or(0) as u16;
                crate::tui::views::copy::render(
                    f,
                    main_rect,
                    copy_list_content,
                    copy_trades_content,
                    s0,
                    s1,
                    focused_section,
                );
            }
            4 => crate::tui::views::portfolio::render(f, main_rect, live, scroll),
            5 => {
                if let Some(dv) = discover_view {
                    crate::tui::views::discover::render(f, main_rect, dv, scroll);
                }
            }
            6 => crate::tui::views::backtester::render(
                f,
                main_rect,
                backtester,
                focused_section,
                backtest_strategy_sel,
                backtest_selected_strategy,
                backtest_data_sel,
                backtest_tool_sel,
                backtest_param_sel,
                backtest_param_editing,
                backtest_param_input,
                backtest_show_graph,
                backtest_show_help,
                backtest_data_dialog,
            ),
            _ => {}
        }

        // Bottom strip background fill
        if is_main_tab {
            let strip_y = trades_rect.y;
            let strip_h = area.height.saturating_sub(strip_y);
            if strip_h > 0 {
                f.render_widget(
                    Block::default()
                        .style(Style::default().bg(Theme::BG()))
                        .borders(Borders::NONE),
                    Rect {
                        x: area.x,
                        y: strip_y,
                        width: area.width,
                        height: strip_h,
                    },
                );
            }
        }

        // Bottom trade strip (two panes)
        if is_main_tab && trades_rect.width > 0 && trades_rect.height > 0 {
            let horz = RLayout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
                .split(trades_rect);
            if tab_index == 3 {
                let status = copy_status_line.unwrap_or(
                    "Monitor: Stopped | Auto-execute: off | Sizing: proportional | Bankroll: —",
                );
                f.render_widget(
                    Paragraph::new(status)
                        .style(Theme::body())
                        .wrap(Wrap { trim: true })
                        .block(crate::tui::views::header::bordered_block(" Copy Status ")),
                    horz[0],
                );
                f.render_widget(
                    Paragraph::new(copy_last_line.unwrap_or("—"))
                        .style(Theme::body())
                        .wrap(Wrap { trim: true })
                        .block(crate::tui::views::header::bordered_block(" Last copy ")),
                    horz[1],
                );
            } else {
                f.render_widget(
                    Paragraph::new("—")
                        .style(Theme::dim())
                        .block(crate::tui::views::header::bordered_block(" Active Trades ")),
                    horz[0],
                );
                f.render_widget(
                    Paragraph::new("—")
                        .style(Theme::dim())
                        .block(crate::tui::views::header::bordered_block(" Closed Trades ")),
                    horz[1],
                );
            }
        }

        // Hint bar
        let indicator_area = chunks[indicator_idx];
        let scan_note = discover_view.map(|dv| dv.scan_note.as_str()).unwrap_or("");
        crate::tui::views::header::render_hint_bar(
            f,
            indicator_area,
            if scan_note.is_empty() {
                None
            } else {
                Some(scan_note)
            },
        );
    }

    // ── Delegating dialog methods (called from runner.rs) ─────────────────────

    pub fn render_shortcuts_screen(f: &mut Frame, shortcuts: &[(String, Vec<ShortcutPair>)]) {
        crate::tui::views::dialogs::render_shortcuts_screen(f, shortcuts);
    }

    #[allow(clippy::too_many_arguments)]
    pub fn render_theme_screen(
        f: &mut Frame,
        theme_selection: usize,
        theme_names: &[String],
        current_theme_index: usize,
        in_creator: bool,
        creator_role: usize,
        creator_color_idx: usize,
        editor_palette: Option<&crate::tui::theme::ThemePalette>,
    ) {
        crate::tui::views::dialogs::render_theme_screen(
            f,
            theme_selection,
            theme_names,
            current_theme_index,
            in_creator,
            creator_role,
            creator_color_idx,
            editor_palette,
        );
    }

    pub fn render_live_confirm(f: &mut Frame, tab_index: usize) {
        crate::tui::views::dialogs::render_live_confirm(f, tab_index);
    }

    pub fn render_discover_filter_dialog(
        f: &mut Frame,
        title: &str,
        options: &[&str],
        selected: usize,
    ) {
        crate::tui::views::dialogs::render_discover_filter_dialog(f, title, options, selected);
    }

    pub fn render_bankroll_prompt(f: &mut Frame, input: &str) {
        crate::tui::views::dialogs::render_bankroll_prompt(f, input);
    }

    pub fn render_currency_picker(
        f: &mut Frame,
        currencies: &[&str],
        selected: usize,
        filter: &str,
    ) {
        crate::tui::views::dialogs::render_currency_picker(f, currencies, selected, filter);
    }

    pub fn render_copy_add_dialog(
        f: &mut Frame,
        search_query: &str,
        option_rows: &[(String, bool)],
    ) {
        crate::tui::views::dialogs::render_copy_add_dialog(f, search_query, option_rows);
    }
}
