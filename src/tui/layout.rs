use ratatui::{
    layout::{Constraint, Direction, Layout as RLayout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Paragraph, Row, Table, Tabs, Wrap},
    Frame,
};

use super::theme::{Theme, ThemePalette, COLOR_PRESETS, THEME_CREATOR_ROLES};
use crate::live::{LiveState, LogLevel};

const TABS: &[&str] = &["Crypto", "Sports", "Weather", "Copy", "Discover"];
const PADDING_H: u16 = 2;
const PADDING_V: u16 = 1;
const TITLE_MARGIN_EXTRA: u16 = 0;
const MIN_TERMINAL_WIDTH: u16 = 60;
const MIN_TERMINAL_HEIGHT: u16 = 24;

pub struct Layout;

/// Shortcut entry for the Shortcuts block: (key, action).
pub type ShortcutPair = (String, String);

/// Discover tab: filters, optional table rows for Leaderboard, scan note, loading.
#[derive(Clone)]
pub struct DiscoverView {
    pub filters: String,
    pub table: String,
    /// Header row cell texts; then row_cells: (selected, roi_positive, cells).
    pub leaderboard_header: Vec<String>,
    pub leaderboard_rows: Vec<(bool, bool, Vec<String>)>,
    pub scan_note: String,
    pub loading: bool,
}

fn inner_area(area: Rect) -> Rect {
    let w = area.width.saturating_sub(2 * PADDING_H);
    let h = area.height.saturating_sub(2 * PADDING_V);
    Rect {
        x: area.x + PADDING_H,
        y: area.y + PADDING_V,
        width: w,
        height: h,
    }
}

fn bordered_block(title: &str) -> Block<'_> {
    Block::default()
        .borders(Borders::ALL)
        .title(format!(" {} ", title))
        .border_style(Theme::block_border())
        .title_style(Theme::block_title())
        .style(Style::default().bg(Theme::PANEL_BG()))
}

fn bordered_block_error(title: &str) -> Block<'_> {
    Block::default()
        .borders(Borders::ALL)
        .title(format!(" {} — Error (see logs) ", title))
        .border_style(Theme::danger())
        .title_style(Theme::block_title())
        .style(Style::default().bg(Theme::PANEL_BG()))
}

/// Count how many lines the shortcuts will take when rendered (category line + wrapped shortcut lines per category).
fn shortcut_lines_count(shortcuts: &[(String, Vec<ShortcutPair>)], width: usize) -> usize {
    let mut total = 0usize;
    for (i, (_category, pairs)) in shortcuts.iter().enumerate() {
        total += 1; // category title
        total += 1; // blank after title
        let mut len = 0usize;
        for (k, a) in pairs {
            let group_len = 2 + k.len() + 2 + a.len() + 3;
            if len + group_len > width && len > 0 {
                total += 1;
                len = 0;
            }
            len += group_len;
        }
        if len > 0 {
            total += 1;
        }
        if i + 1 < shortcuts.len() {
            total += 2; // divider line + blank
        }
    }
    total
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

impl Layout {
    pub fn render(
        f: &mut Frame,
        selected_tab: usize,
        copy_tab_content: &str,
        discover_content: &str,
        discover_view: Option<&DiscoverView>,
        live: &LiveState,
        shortcuts: &[(String, Vec<ShortcutPair>)],
        tab_mode: Option<&str>,
    ) {
        let area = f.area();
        let bg = Block::default().style(Style::default().bg(Theme::BG()));
        f.render_widget(bg, area);

        if area.width < MIN_TERMINAL_WIDTH || area.height < MIN_TERMINAL_HEIGHT {
            let messages = dimensions_too_small_messages();
            let idx = (area.width as usize + area.height as usize) % messages.len();
            let sub = messages[idx];
            let inner = inner_area(area);
            let para = Paragraph::new(vec![
                Line::from(""),
                Line::from(Span::styled(
                    "Myoro Polymarket Terminal",
                    Theme::block_title(),
                )),
                Line::from(""),
                Line::from(Span::styled(sub, Theme::body())),
            ])
            .alignment(ratatui::layout::Alignment::Center);
            f.render_widget(para, inner);
            return;
        }

        let inner = inner_area(area);
        if inner.width == 0 || inner.height == 0 {
            return;
        }
        let width = inner.width as usize;
        let shortcut_line_count = shortcut_lines_count(shortcuts, width);
        let shortcut_height = (shortcut_line_count + 2).min(inner.height as usize) as u16;

        let tab_index = selected_tab.min(TABS.len().saturating_sub(1));
        let log_height = if tab_index == 4 { 0 } else { 8 };
        let chunks = RLayout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Length(3),
                Constraint::Length(3),
                Constraint::Min(4),
                Constraint::Length(log_height),
                Constraint::Length(shortcut_height),
            ])
            .margin(0)
            .split(inner);

        let title_area = Rect {
            x: chunks[0].x + TITLE_MARGIN_EXTRA,
            y: chunks[0].y,
            width: chunks[0].width.saturating_sub(2 * TITLE_MARGIN_EXTRA),
            height: chunks[0].height,
        };
        let header_block = Block::default()
            .borders(Borders::ALL)
            .border_style(Theme::block_border())
            .style(Style::default().bg(Theme::PANEL_BG()));
        let inner = header_block.inner(title_area);
        f.render_widget(header_block, title_area);
        let title_center = " Myoro Polymarket Terminal ";
        let w = inner.width;
        let n = title_center.len() as u16;
        let left = (w.saturating_sub(n)) / 2;
        let right = w.saturating_sub(n).saturating_sub(left);
        let line_str: String =
            "─".repeat(left as usize) + title_center + &"─".repeat(right as usize);
        let header_para = Paragraph::new(Line::from(Span::styled(
            line_str,
            Theme::block_title(),
        )));
        f.render_widget(header_para, inner);

        let stats = live.global_stats.read().ok();
        let bankroll = stats
            .as_ref()
            .and_then(|s| s.bankroll)
            .map(|b| format!("{:.2}", b))
            .unwrap_or_else(|| "—".to_string());
        let pnl = stats.as_ref().map(|s| s.pnl).unwrap_or(0.0);
        let open_t = stats.as_ref().map(|s| s.open_trades).unwrap_or(0);
        let closed_t = stats.as_ref().map(|s| s.closed_trades).unwrap_or(0);
        let pnl_str = format!("{:.2}", pnl);
        let pnl_style = if pnl > 0.0 {
            Theme::success()
        } else if pnl < 0.0 {
            Theme::danger()
        } else {
            Theme::neutral_pnl()
        };
        let mode_str = tab_mode.map(|m| format!("  |  Mode: {} ", m)).unwrap_or_default();
        let stats_line = Line::from(vec![
            Span::raw("  P&L: "),
            Span::styled(&pnl_str, pnl_style),
            Span::raw(format!(
                "  |  Bankroll: {}  |  Open: {}  |  Closed: {}{}",
                bankroll, open_t, closed_t, mode_str
            )),
        ]);
        let stats_para = Paragraph::new(stats_line)
            .block(bordered_block("Metrics"));
        f.render_widget(stats_para, chunks[1]);

        let tab_index = selected_tab.min(TABS.len().saturating_sub(1));
        let tab_titles = TABS.iter().map(|t| Line::from(*t)).collect::<Vec<_>>();
        let tabs = Tabs::new(tab_titles)
            .block(bordered_block("Tabs"))
            .style(Theme::tab_default())
            .highlight_style(Theme::tab_selected())
            .select(tab_index);
        f.render_widget(tabs, chunks[2]);

        let content_chunk = chunks[3];
        match tab_index {
            0 => {
                let c = live.crypto.read().ok();
                let btc_raw = c.as_ref().map(|c| c.btc_usdt.as_str()).unwrap_or("");
                let btc = if btc_raw.is_empty() || btc_raw == "—" {
                    "⏳ Loading…\n(Background fetch every 8s.)"
                } else {
                    btc_raw
                };
                let events = c
                    .as_ref()
                    .map(|c| c.events.join("\n"))
                    .unwrap_or_default();
                let sub = RLayout::default()
                    .direction(Direction::Vertical)
                    .constraints([Constraint::Length(2), Constraint::Min(4)])
                    .split(content_chunk);
                let news_block = if live.last_log_is_error(0) {
                    bordered_block_error("News & Data")
                } else {
                    bordered_block("News & Data")
                };
                let news = Paragraph::new(btc)
                    .style(Theme::body())
                    .wrap(Wrap { trim: true })
                    .block(news_block);
                f.render_widget(news, sub[0]);
                let events_block = Paragraph::new(events)
                    .style(Theme::body())
                    .wrap(Wrap { trim: true })
                    .block(bordered_block("Active events (Gamma)"));
                f.render_widget(events_block, sub[1]);
            }
            1 => {
                let s = live.sports.read().ok();
                let lines = s
                    .as_ref()
                    .map(|s| s.fixtures.join("\n"))
                    .unwrap_or_default();
                let sub = RLayout::default()
                    .direction(Direction::Vertical)
                    .constraints([Constraint::Length(2), Constraint::Min(4)])
                    .split(content_chunk);
                let strategies = Paragraph::new(
                    "• Premier League fixtures (FBRef / Fixture Download)\n\n\
                     Add more in config; see docs for arbitrage/value strategies."
                )
                    .style(Theme::body())
                    .wrap(Wrap { trim: true })
                    .block(bordered_block("Strategies"));
                f.render_widget(strategies, sub[0]);
                let fixture_content = if lines.is_empty() {
                    "⏳ Loading…".to_string()
                } else {
                    format!("Premier League\n\n{}", lines)
                };
                let fixtures_block = if live.last_log_is_error(1) {
                    bordered_block_error("Fixtures")
                } else {
                    bordered_block("Fixtures")
                };
                let fixtures = Paragraph::new(fixture_content)
                    .style(Theme::body())
                    .wrap(Wrap { trim: true })
                    .block(fixtures_block);
                f.render_widget(fixtures, sub[1]);
            }
            2 => {
                let w = live.weather.read().ok();
                let lines = w
                    .as_ref()
                    .map(|w| w.forecast.join("\n"))
                    .unwrap_or_default();
                let content = if lines.is_empty() {
                    "⏳ Loading…".to_string()
                } else {
                    format!("7-day forecast (NYC)\n\n{}", lines)
                };
                let forecast_block = if live.last_log_is_error(2) {
                    bordered_block_error("Forecast")
                } else {
                    bordered_block("Forecast")
                };
                let forecast = Paragraph::new(content)
                    .style(Theme::body())
                    .wrap(Wrap { trim: true })
                    .block(forecast_block);
                f.render_widget(forecast, content_chunk);
            }
            3 => {
                let copy_block = Paragraph::new(copy_tab_content)
                    .style(Theme::body())
                    .wrap(Wrap { trim: true })
                    .block(bordered_block("Copy List"));
                f.render_widget(copy_block, content_chunk);
            }
            4 => {
                if let Some(dv) = discover_view {
                    let sub = RLayout::default()
                        .direction(Direction::Vertical)
                        .constraints([Constraint::Length(8), Constraint::Min(4)])
                        .split(content_chunk);
                    let mut filter_lines: Vec<Line> = Vec::new();
                    for line in dv.filters.lines() {
                        if line == "Category" || line == "Period" || line == "Order" {
                            filter_lines.push(Line::from(Span::styled(line, Theme::block_title())));
                        } else {
                            filter_lines.push(Line::from(Span::styled(line, Theme::body())));
                        }
                    }
                    let filters_para = Paragraph::new(filter_lines)
                        .wrap(Wrap { trim: true })
                        .block(bordered_block("Filters"));
                    f.render_widget(filters_para, sub[0]);
                    if dv.loading && dv.leaderboard_rows.is_empty() {
                        let p = Paragraph::new("⏳ Refreshing…")
                            .style(Theme::body())
                            .block(bordered_block("Leaderboard"));
                        f.render_widget(p, sub[1]);
                    } else if !dv.leaderboard_rows.is_empty() {
                        let widths = [
                            Constraint::Length(2),
                            Constraint::Length(4),
                            Constraint::Length(12),
                            Constraint::Length(10),
                            Constraint::Length(10),
                            Constraint::Length(6),
                            Constraint::Length(6),
                            Constraint::Length(10),
                            Constraint::Length(12),
                        ];
                        let header_cells = dv.leaderboard_header.iter()
                            .map(|s| Cell::from(s.as_str()).style(Theme::block_title()));
                        let header = Row::new(header_cells).height(1);
                        let rows: Vec<Row> = dv.leaderboard_rows.iter()
                            .map(|(selected, roi_pos, cells)| {
                                let cells: Vec<Cell> = cells.iter().enumerate().map(|(i, s)| {
                                    let style = if *selected {
                                        Theme::tab_selected()
                                    } else if i == 5 && *roi_pos {
                                        Theme::success()
                                    } else {
                                        Theme::body()
                                    };
                                    Cell::from(s.as_str()).style(style)
                                }).collect();
                                Row::new(cells).height(1)
                            })
                            .collect();
                        let table = Table::new(rows, widths)
                            .header(header)
                            .block(bordered_block("Leaderboard"))
                            .column_spacing(1);
                        f.render_widget(table, sub[1]);
                        if !dv.scan_note.is_empty() {
                            let note_rect = Rect {
                                x: sub[1].x,
                                y: sub[1].y + sub[1].height.saturating_sub(1),
                                width: sub[1].width,
                                height: 1,
                            };
                            let note = Paragraph::new(Span::styled(
                                &dv.scan_note,
                                Theme::dim().add_modifier(Modifier::BOLD),
                            ));
                            f.render_widget(note, note_rect);
                        }
                    } else {
                        let p = Paragraph::new(dv.table.as_str())
                            .style(Theme::body())
                            .block(bordered_block("Leaderboard"));
                        f.render_widget(p, sub[1]);
                    }
                } else {
                    let fallback = Paragraph::new(discover_content)
                        .style(Theme::body())
                        .wrap(Wrap { trim: true })
                        .block(bordered_block("Leaderboard"));
                    f.render_widget(fallback, content_chunk);
                }
            }
            _ => {}
        }

        if log_height > 0 {
            let tab_log_title = format!("{} Logs", TABS.get(tab_index).unwrap_or(&""));
            let per_tab_logs = match tab_index {
                0 => live.get_crypto_logs(),
                1 => live.get_sports_logs(),
                2 => live.get_weather_logs(),
                3 => live.get_copy_logs(),
                4 => Vec::new(),
                _ => Vec::new(),
            };
            let log_style = |l: LogLevel| match l {
                LogLevel::Success => Theme::success(),
                LogLevel::Warning => Theme::warning(),
                LogLevel::Error => Theme::danger(),
                LogLevel::Info => Theme::dim(),
            };
            let log_lines: Vec<Line> = if per_tab_logs.is_empty() {
                vec![Line::from(Span::styled("—", Theme::dim()))]
            } else {
                per_tab_logs
                    .into_iter()
                    .map(|(level, msg)| Line::from(Span::styled(msg, log_style(level))))
                    .collect()
            };
            let log_para = Paragraph::new(log_lines)
                .wrap(Wrap { trim: true })
                .block(bordered_block(&tab_log_title));
            f.render_widget(log_para, chunks[4]);
        }

        let shortcut_area = chunks[5];
        let width = shortcut_area.width as usize;
        let mut lines: Vec<Line> = Vec::new();
        for (i, (category, pairs)) in shortcuts.iter().enumerate() {
            if i > 0 {
                let div = "─".repeat(width.min(40));
                let pad = width.saturating_sub(div.len()) / 2;
                lines.push(Line::from(Span::raw(" ".repeat(pad) + &div)));
                lines.push(Line::from(""));
            }
            let cat_len = category.len();
            let pad = width.saturating_sub(cat_len) / 2;
            let mut cat_line = vec![Span::raw(" ".repeat(pad))];
            cat_line.push(Span::styled(
                category.as_str(),
                Theme::block_title(),
            ));
            lines.push(Line::from(cat_line));
            lines.push(Line::from(""));

            let mut line_spans: Vec<Span> = Vec::new();
            let mut len = 0usize;
            for (k, a) in pairs {
                let group_len = 2 + k.len() + 2 + a.len() + 3;
                if len + group_len > width && !line_spans.is_empty() {
                    let content_len: usize = line_spans.iter().map(|s| s.content.len()).sum();
                    let pad = width.saturating_sub(content_len) / 2;
                    let mut centered = vec![Span::raw(" ".repeat(pad))];
                    centered.append(&mut line_spans);
                    lines.push(Line::from(centered));
                    len = 0;
                }
                line_spans.push(Span::styled(format!("[{}] ", k), Theme::key()));
                line_spans.push(Span::styled(format!("{}   ", a), Theme::body()));
                len += group_len;
            }
            if !line_spans.is_empty() {
                let content_len: usize = line_spans.iter().map(|s| s.content.len()).sum();
                let pad = width.saturating_sub(content_len) / 2;
                let mut centered = vec![Span::raw(" ".repeat(pad))];
                centered.append(&mut line_spans);
                lines.push(Line::from(centered));
            }
        }
        let shortcuts_para = Paragraph::new(lines).block(bordered_block("Shortcuts"));
        f.render_widget(shortcuts_para, shortcut_area);
    }

    /// Floating confirmation when switching to Live mode.
    pub fn render_live_confirm(f: &mut Frame, tab_index: usize) {
        let area = f.area();
        let tab_name = TABS.get(tab_index).copied().unwrap_or("?");
        let msg = format!("Switch {} to Live trading? (y/n)", tab_name);
        let bw = (msg.len() as u16 + 4).max(24).min(area.width);
        let bh = 5u16;
        let x = area.x + area.width.saturating_sub(bw) / 2;
        let y = area.y + area.height.saturating_sub(bh) / 2;
        let rect = Rect { x, y, width: bw, height: bh };
        let block = Block::default()
            .borders(Borders::ALL)
            .title(" Confirm ")
            .border_style(Theme::block_border())
            .title_style(Theme::block_title())
            .style(Style::default().bg(Theme::PANEL_BG()));
        let inner = block.inner(rect);
        f.render_widget(block, rect);
        let para = Paragraph::new(msg.as_str()).style(Theme::body());
        f.render_widget(para, inner);
    }

    /// Full-screen theming view (replaces main UI when open).
    pub fn render_theme_screen(
        f: &mut Frame,
        theme_selection: usize,
        theme_names: &[String],
        current_theme_index: usize,
        in_creator: bool,
        creator_role: usize,
        creator_color_idx: usize,
        editor_palette: Option<&ThemePalette>,
    ) {
        Self::render_theme_overlay(
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

    /// Fullscreen overlay for theming (T to open/close).
    pub fn render_theme_overlay(
        f: &mut Frame,
        theme_selection: usize,
        theme_names: &[String],
        current_theme_index: usize,
        in_creator: bool,
        creator_role: usize,
        creator_color_idx: usize,
        editor_palette: Option<&ThemePalette>,
    ) {
        let area = f.area();
        let bg = Block::default().style(Style::default().bg(Theme::BG()));
        f.render_widget(bg, area);
        let inner = inner_area(area);
        if inner.width == 0 || inner.height == 0 {
            return;
        }
        let block = Block::default()
            .borders(Borders::ALL)
            .title(" Theming — T / Esc close ")
            .border_style(Theme::block_border())
            .title_style(Theme::block_title())
            .style(Style::default().bg(Theme::PANEL_BG()));
        let block_inner = block.inner(inner);
        f.render_widget(block, inner);

        let mut lines: Vec<Line> = Vec::new();
        if in_creator {
            lines.push(Line::from("Theme Creator — j/k role, h/l color, s save, Esc back"));
            lines.push(Line::from(""));
            if let Some(p) = editor_palette {
                for (i, name) in THEME_CREATOR_ROLES.iter().enumerate() {
                    let mark = if i == creator_role { "► " } else { "  " };
                    let c = p.role_color(i);
                    let color_preview = format!(" [{:3},{:3},{:3}] ", c[0], c[1], c[2]);
                    lines.push(Line::from(format!("{}{}: {}", mark, name, color_preview)));
                }
                lines.push(Line::from(""));
                let ci = creator_color_idx.min(COLOR_PRESETS.len().saturating_sub(1));
                let cp = COLOR_PRESETS[ci];
                lines.push(Line::from(format!("Color preset {}/{}: {:?}  (h/l change)", ci + 1, COLOR_PRESETS.len(), cp)));
            }
        } else {
            lines.push(Line::from("↑↓/jk select  Enter apply  e Export  i Import  c Creator"));
            lines.push(Line::from(""));
            for (i, name) in theme_names.iter().enumerate() {
                let mark = if i == theme_selection { "► " } else { "  " };
                let cur = if i == current_theme_index { " (active)" } else { "" };
                lines.push(Line::from(format!("{}{}{}", mark, name, cur)));
            }
            lines.push(Line::from(""));
            let export_sel = theme_selection == theme_names.len();
            let import_sel = theme_selection == theme_names.len() + 1;
            let creator_sel = theme_selection == theme_names.len() + 2;
            lines.push(Line::from(format!("{}[e] Export to theme_export.toml", if export_sel { "► " } else { "  " })));
            lines.push(Line::from(format!("{}[i] Import from theme_import.toml", if import_sel { "► " } else { "  " })));
            lines.push(Line::from(format!("{}[c] Theme creator", if creator_sel { "► " } else { "  " })));
        }
        let para = Paragraph::new(lines).style(Theme::body());
        f.render_widget(para, block_inner);
    }

    pub fn render_bankroll_prompt(f: &mut Frame, input: &str) {
        let area = f.area();
        let w = 50u16.min(area.width);
        let h = 5u16;
        let x = area.x + area.width.saturating_sub(w) / 2;
        let y = area.y + area.height.saturating_sub(h) / 2;
        let rect = Rect { x, y, width: w, height: h };
        let block = Block::default()
            .borders(Borders::ALL)
            .title(" Set paper bankroll ")
            .border_style(Theme::block_border())
            .title_style(Theme::block_title())
            .style(Style::default().bg(Theme::PANEL_BG()));
        let inner = block.inner(rect);
        f.render_widget(block, rect);
        let msg = format!("Amount: {}_\nEnter confirm  Esc cancel", input);
        let para = Paragraph::new(msg).style(Theme::body());
        f.render_widget(para, inner);
    }
}
