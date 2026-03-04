// NOTE: Exceeds 300-line limit — leaderboard and screener share the same tab area; combining both modes avoids splitting across files with tightly coupled render logic. See docs/ai-rules/file-size.md
//! Discover tab renderer (leaderboard + screener mode) and view-model builder.

use crate::discover::{DiscoverState, LeaderboardEntry};
use crate::tui::layout::DiscoverView;
use crate::tui::theme::Theme;
use crate::tui::views::header::bordered_block;
use ratatui::{
    layout::{Constraint, Direction, Layout as RLayout, Rect},
    text::{Line, Span},
    widgets::{Cell, Paragraph, Row, Table},
    Frame,
};

// ── View-model builder ────────────────────────────────────────────────────────

const W_USER: usize = 12;
const W_MAINLY: usize = 10;

fn truncate_user(s: &str, max: usize) -> &str {
    if s.len() <= max {
        s
    } else {
        &s[..max]
    }
}

#[allow(clippy::too_many_arguments)]
pub fn build_view(
    entries: &[LeaderboardEntry],
    selected: Option<usize>,
    discover: &DiscoverState,
    copy_addresses: &[String],
    max_rows: usize,
) -> DiscoverView {
    let loading = discover.is_fetching();
    let filters = (
        discover.category_label(),
        discover.time_period_label(),
        discover.order_by_label(),
    );
    let (table, header, rows, scan_note) = if entries.is_empty() && !loading {
        (
            "No data. Press r to fetch.".to_string(),
            vec![],
            vec![],
            String::new(),
        )
    } else if entries.is_empty() {
        (String::new(), vec![], vec![], String::new())
    } else {
        let total = entries.len();
        let page_size = max_rows.min(total).max(1);
        let sel = selected.unwrap_or(0).min(total.saturating_sub(1));
        let start = (sel as i32 - (page_size as i32 / 2))
            .max(0)
            .min((total.saturating_sub(page_size)) as i32) as usize;
        let end = (start + page_size).min(total);
        let header = vec![
            "".to_string(),
            "Rank".to_string(),
            "User".to_string(),
            "Vol".to_string(),
            "P&L".to_string(),
            "ROI%".to_string(),
            "Trades".to_string(),
            "Mainly".to_string(),
            "Address".to_string(),
        ];
        let is_copied = |addr: &str| copy_addresses.iter().any(|a| a == addr);
        let mut row_data = Vec::new();
        for (idx, e) in entries[start..end].iter().enumerate() {
            let global_idx = start + idx;
            let selected_row = Some(global_idx) == selected;
            let copied = is_copied(&e.proxy_wallet);
            let roi = if e.vol > 0.0 {
                e.pnl / e.vol * 100.0
            } else {
                0.0
            };
            let stats = discover.get_stats(&e.proxy_wallet);
            let (trades, mainly) = stats
                .map(|s| (s.trade_count.to_string(), s.top_category.clone()))
                .unwrap_or_else(|| ("…".to_string(), "…".to_string()));
            let mainly_short = if mainly.len() > W_MAINLY {
                format!("{}…", &mainly[..W_MAINLY.saturating_sub(1)])
            } else {
                mainly.clone()
            };
            let addr_short = e
                .proxy_wallet
                .get(..10)
                .map(|s| format!("{}…", s))
                .unwrap_or_else(|| e.proxy_wallet.clone());
            let sel_mark = if selected_row { "►" } else { " " };
            let copy_mark = if copied { " ●" } else { "" };
            let cells = vec![
                format!("{}{}", sel_mark, copy_mark),
                e.rank.clone(),
                truncate_user(&e.user_name, W_USER).to_string(),
                format!("{:.2}", e.vol),
                format!("{:.2}", e.pnl),
                format!("{:.1}%", roi),
                trades,
                mainly_short,
                addr_short,
            ];
            row_data.push((selected_row, roi > 0.0, copied, cells));
        }
        let table_str = format!(
            "Profiles: {} (showing {}-{})   ↑↓ / jk  scroll   a / Enter  add to copy",
            total,
            start + 1,
            end
        );
        (
            table_str,
            header,
            row_data,
            "Background scan: Trades + Mainly fill in as profiles are fetched.".to_string(),
        )
    };
    DiscoverView {
        filters_category: filters.0,
        filters_period: filters.1,
        filters_order: filters.2,
        table,
        leaderboard_header: header,
        leaderboard_rows: rows,
        scan_note,
        loading,
        screener_mode: discover.is_screener_mode(),
        screener_markets: discover.get_screener_markets(),
    }
}

// ── Renderer ─────────────────────────────────────────────────────────────────

pub fn render(f: &mut Frame, area: Rect, dv: &DiscoverView, scroll_offset: usize) {
    let sub = RLayout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(0)])
        .split(area);
    if dv.screener_mode {
        render_screener(f, sub[0], sub[1], dv, scroll_offset);
    } else {
        render_leaderboard(f, sub[0], sub[1], dv, scroll_offset);
    }
}

fn render_screener(
    f: &mut Frame,
    header_area: Rect,
    body_area: Rect,
    dv: &DiscoverView,
    scroll_offset: usize,
) {
    f.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("[s] ", Theme::key()),
            Span::styled("Back to Leaderboard  ", Theme::body()),
            Span::styled("[r] ", Theme::key()),
            Span::styled("Refresh Screener", Theme::block_title()),
        ]))
        .block(bordered_block("Market Screener")),
        header_area,
    );
    if dv.screener_markets.is_empty() {
        f.render_widget(
            Paragraph::new("⏳ Fetching markets…")
                .style(Theme::body())
                .block(bordered_block("Markets by Edge Score")),
            body_area,
        );
        return;
    }
    let header = Row::new([
        Cell::from("Score").style(Theme::block_title()),
        Cell::from("Spread").style(Theme::block_title()),
        Cell::from("Vol").style(Theme::block_title()),
        Cell::from("Market").style(Theme::block_title()),
    ]);
    let visible = body_area.height.saturating_sub(2) as usize;
    let end = (scroll_offset + visible).min(dv.screener_markets.len());
    let rows: Vec<Row> = dv.screener_markets[scroll_offset..end]
        .iter()
        .map(|m| {
            let title_short = m.title.get(..30).unwrap_or(&m.title);
            Row::new([
                Cell::from(format!("{:.2}", m.edge_score)).style(Theme::success()),
                Cell::from(format!("{:.3}", m.spread)).style(Theme::body()),
                Cell::from(format!("{:.0}", m.volume)).style(Theme::body()),
                Cell::from(title_short).style(Theme::body()),
            ])
        })
        .collect();
    f.render_widget(
        Table::new(
            rows,
            [
                Constraint::Length(6),
                Constraint::Length(6),
                Constraint::Length(8),
                Constraint::Min(10),
            ],
        )
        .header(header)
        .block(bordered_block("Markets by Edge Score")),
        body_area,
    );
}

fn render_leaderboard(
    f: &mut Frame,
    filter_area: Rect,
    table_area: Rect,
    dv: &DiscoverView,
    scroll_offset: usize,
) {
    f.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("[c] ", Theme::key()),
            Span::styled("Category ", Theme::block_title()),
            Span::raw(format!("{}  ", dv.filters_category)),
            Span::styled("[t] ", Theme::key()),
            Span::styled("Period ", Theme::block_title()),
            Span::raw(format!("{}  ", dv.filters_period)),
            Span::styled("[o] ", Theme::key()),
            Span::styled("Order ", Theme::block_title()),
            Span::raw(format!("{}  ", dv.filters_order)),
            Span::styled("[r] ", Theme::key()),
            Span::styled("Refresh  ", Theme::block_title()),
            Span::styled("[s] ", Theme::key()),
            Span::styled("Screener", Theme::block_title()),
        ]))
        .block(bordered_block("Filters")),
        filter_area,
    );
    if dv.loading && dv.leaderboard_rows.is_empty() {
        f.render_widget(
            Paragraph::new("⏳ Refreshing…")
                .style(Theme::body())
                .block(bordered_block("Leaderboard")),
            table_area,
        );
        return;
    }
    if dv.leaderboard_rows.is_empty() {
        f.render_widget(
            Paragraph::new(dv.table.as_str())
                .style(Theme::body())
                .block(bordered_block("Leaderboard")),
            table_area,
        );
        return;
    }
    let visible = table_area.height.saturating_sub(1) as usize;
    let end = (scroll_offset + visible).min(dv.leaderboard_rows.len());
    let widths = [
        Constraint::Length(4),
        Constraint::Length(4),
        Constraint::Min(8),
        Constraint::Min(8),
        Constraint::Min(6),
        Constraint::Length(6),
        Constraint::Length(6),
        Constraint::Min(8),
        Constraint::Min(10),
    ];
    let header = Row::new(
        dv.leaderboard_header
            .iter()
            .map(|s| Cell::from(s.as_str()).style(Theme::block_title())),
    )
    .height(1);
    let rows: Vec<Row> = dv.leaderboard_rows[scroll_offset..end]
        .iter()
        .enumerate()
        .map(|(idx, (selected, roi_pos, copied, cells))| {
            let sel = idx == 0;
            let row_cells: Vec<Cell> = cells
                .iter()
                .enumerate()
                .map(|(i, s)| {
                    let style = if sel || *selected {
                        Theme::tab_selected()
                    } else if *copied || (i == 5 && *roi_pos) {
                        Theme::success()
                    } else {
                        Theme::body()
                    };
                    Cell::from(s.as_str()).style(style)
                })
                .collect();
            Row::new(row_cells).height(1)
        })
        .collect();
    f.render_widget(
        Table::new(rows, widths)
            .header(header)
            .block(bordered_block("Leaderboard"))
            .column_spacing(1),
        table_area,
    );
}
