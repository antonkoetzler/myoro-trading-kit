// NOTE: Exceeds 300-line limit — three-pane sports renderer with embedded view-model builder; splitting would require passing many sub-structs across modules. See docs/ai-rules/file-size.md
//! Sports tab 3-pane renderer and view-model builder.

use crate::live::LiveState;
use crate::tui::layout::{FixtureRow, SignalRow, SportsView, StrategyRow};
use crate::tui::theme::Theme;
use crate::tui::views::header::bordered_block;
use ratatui::{
    layout::{Constraint, Direction, Layout as RLayout, Rect},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

pub fn render(f: &mut Frame, area: Rect, sv: &SportsView, scroll: usize) {
    let cols = RLayout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(22),
            Constraint::Percentage(38),
            Constraint::Percentage(40),
        ])
        .split(area);

    render_strategies(f, cols[0], &sv.strategies, sv.pane);
    render_signals(f, cols[1], &sv.signals, sv.pane, sv.pending_count, scroll);
    render_fixtures(
        f,
        cols[2],
        &sv.fixtures,
        sv.pane,
        scroll,
        sv.league_filter.as_deref(),
        sv.team_filter.as_deref(),
    );

    if sv.show_league_picker {
        render_picker_overlay(
            f,
            " League Filter — Enter apply, Esc cancel ",
            &sv.league_picker_items,
            sv.league_picker_sel,
        );
    } else if sv.show_team_picker {
        render_picker_overlay(
            f,
            " Team Filter — Enter apply, Esc cancel ",
            &sv.team_picker_items,
            sv.team_picker_sel,
        );
    }
}

fn render_strategies(f: &mut Frame, area: Rect, strategies: &[StrategyRow], pane: usize) {
    let title = if pane == 0 {
        "► Strategies"
    } else {
        "  Strategies"
    };
    let block = bordered_block(title);
    let inner = block.inner(area);
    f.render_widget(block, area);

    let mut lines: Vec<Line> = strategies
        .iter()
        .map(|s| {
            let prefix = if s.is_custom {
                "[c]"
            } else if s.enabled {
                "[x]"
            } else {
                "[ ]"
            };
            let style = if s.selected && pane == 0 {
                Theme::tab_selected()
            } else if s.enabled {
                Theme::success()
            } else {
                Theme::body()
            };
            Line::from(Span::styled(format!("{} {}", prefix, s.name), style))
        })
        .collect();

    if inner.height > strategies.len() as u16 + 1 {
        lines.push(Line::from(""));
        lines.push(Line::from(vec![
            Span::styled("[Space]", Theme::key()),
            Span::raw(" toggle"),
        ]));
    }
    f.render_widget(Paragraph::new(lines).style(Theme::body()), inner);
}

fn render_signals(
    f: &mut Frame,
    area: Rect,
    signals: &[SignalRow],
    pane: usize,
    pending_count: usize,
    scroll: usize,
) {
    let feed_title = if pane == 1 {
        format!("► Signals ({} pending)", pending_count)
    } else {
        format!("  Signals ({} pending)", pending_count)
    };
    let block = bordered_block(&feed_title);
    let inner = block.inner(area);
    f.render_widget(block, area);

    let done_idx = signals.iter().position(|s| s.status != "pending");
    let mut lines: Vec<Line> = Vec::new();
    for (i, sig) in signals.iter().enumerate() {
        if done_idx == Some(i) {
            lines.push(Line::from(Span::styled(
                "─ executed ─────────────────",
                Theme::dim(),
            )));
        }
        let style = if sig.selected && pane == 1 {
            Theme::tab_selected()
        } else {
            match sig.status.as_str() {
                "pending" => Theme::body(),
                "auto" | "done" => Theme::success(),
                "dismissed" => Theme::dim(),
                _ => Theme::body(),
            }
        };
        lines.push(Line::from(Span::styled(
            format!(
                "{:<12} {:3}  {:>3.0}%  {:>5.2}  [{}]",
                truncate(&sig.team, 12),
                sig.side,
                sig.edge_pct * 100.0,
                sig.kelly_size * 1000.0,
                sig.status,
            ),
            style,
        )));
    }
    if signals.is_empty() {
        lines.push(Line::from(Span::styled(
            "No signals yet. Enable a strategy.",
            Theme::dim(),
        )));
    }

    let visible_h = inner.height as usize;
    let total = lines.len();
    let offset = if pane == 1 {
        scroll.min(total.saturating_sub(visible_h))
    } else {
        0
    };
    let mut visible: Vec<Line> = lines.into_iter().skip(offset).take(visible_h).collect();

    if inner.height > total as u16 + 1 && pending_count > 0 {
        visible.push(Line::from(""));
        visible.push(Line::from(vec![
            Span::styled("[Enter]", Theme::key()),
            Span::raw(" exec  "),
            Span::styled("[d]", Theme::key()),
            Span::raw(" dismiss"),
        ]));
    }
    f.render_widget(Paragraph::new(visible).style(Theme::body()), inner);
}

fn render_fixtures(
    f: &mut Frame,
    area: Rect,
    fixtures: &[FixtureRow],
    pane: usize,
    scroll: usize,
    league_filter: Option<&str>,
    team_filter: Option<&str>,
) {
    let league_label = league_filter.unwrap_or("All");
    let team_label = team_filter.unwrap_or("All");
    let fix_title = format!(
        "{}Fixtures  League: {} | Team: {}",
        if pane == 2 { "► " } else { "  " },
        league_label,
        team_label
    );
    let block = bordered_block(&fix_title);
    let inner = block.inner(area);
    f.render_widget(block, area);

    let visible_h = inner.height as usize;
    let fix_offset = if pane == 2 {
        scroll.min(fixtures.len().saturating_sub(visible_h))
    } else {
        0
    };

    let mut lines: Vec<Line> = fixtures
        .iter()
        .skip(fix_offset)
        .take(visible_h)
        .map(|row| {
            if row.is_date_header {
                return Line::from(Span::styled(row.date.clone(), Theme::block_title()));
            }
            let market = if row.has_market { " [mkt]" } else { "" };
            let style = if row.selected && pane == 2 {
                Theme::tab_selected()
            } else if row.has_market {
                Theme::success()
            } else {
                Theme::body()
            };
            Line::from(Span::styled(
                format!(
                    "  {:>14} vs {:<14}{}",
                    truncate(&row.home, 14),
                    truncate(&row.away, 14),
                    market
                ),
                style,
            ))
        })
        .collect();

    if lines.is_empty() {
        lines.push(Line::from(Span::styled("⏳ Loading…", Theme::dim())));
    }
    if inner.height > fixtures.len() as u16 + 1 {
        lines.push(Line::from(""));
        lines.push(Line::from(vec![
            Span::styled("[L]", Theme::key()),
            Span::raw(" league  "),
            Span::styled("[T]", Theme::key()),
            Span::raw(" team  "),
            Span::styled("[r]", Theme::key()),
            Span::raw(" refresh"),
        ]));
    }
    f.render_widget(Paragraph::new(lines).style(Theme::body()), inner);
}

fn render_picker_overlay(f: &mut Frame, title: &str, items: &[String], sel: usize) {
    let area = f.area();
    let content_w = items.iter().map(|s| s.len()).max().unwrap_or(16).max(24) as u16;
    let content_h = items.len().min(20) as u16;
    let rect_w = (content_w + 4).min(area.width);
    let rect_h = (content_h + 2).min(area.height);
    let rect = ratatui::layout::Rect {
        x: area.x + area.width.saturating_sub(rect_w) / 2,
        y: area.y + area.height.saturating_sub(rect_h) / 2,
        width: rect_w,
        height: rect_h,
    };
    let block = Block::default()
        .borders(Borders::ALL)
        .title(title)
        .border_style(Theme::block_border())
        .title_style(Theme::block_title())
        .style(ratatui::style::Style::default().bg(Theme::BG()));
    let inner = block.inner(rect);
    f.render_widget(block, rect);
    let lines: Vec<Line> = items
        .iter()
        .enumerate()
        .take(inner.height as usize)
        .map(|(i, s)| {
            let style = if i == sel {
                Theme::tab_selected()
            } else {
                Theme::body()
            };
            Line::from(Span::styled(s.as_str(), style))
        })
        .collect();
    f.render_widget(Paragraph::new(lines).style(Theme::body()), inner);
}

fn truncate(s: &str, max: usize) -> &str {
    if s.len() <= max {
        s
    } else {
        &s[..max]
    }
}

// ── View-model builder ────────────────────────────────────────────────────────

#[allow(clippy::too_many_arguments)]
pub fn build_view(
    live: &LiveState,
    pane: usize,
    strategy_sel: usize,
    signal_sel: usize,
    fixture_sel: usize,
    league_filter: Option<&str>,
    team_filter: Option<&str>,
    show_league_picker: bool,
    show_team_picker: bool,
    league_picker_sel: usize,
    team_picker_sel: usize,
) -> SportsView {
    let s = live.sports.read().ok();

    let strategies: Vec<StrategyRow> = s
        .as_ref()
        .map(|st| {
            st.strategy_configs
                .iter()
                .enumerate()
                .map(|(i, c)| StrategyRow {
                    id: c.id.to_string(),
                    name: c.name.to_string(),
                    enabled: c.enabled,
                    is_custom: c.is_custom,
                    selected: i == strategy_sel,
                })
                .collect()
        })
        .unwrap_or_default();

    let signals: Vec<SignalRow> = s
        .as_ref()
        .map(|st| {
            st.signals
                .iter()
                .enumerate()
                .map(|(i, sig)| SignalRow {
                    team: sig.home.clone(),
                    side: sig.side.clone(),
                    edge_pct: sig.edge_pct,
                    kelly_size: sig.kelly_size,
                    status: sig.status.clone(),
                    strategy_id: sig.strategy_id.clone(),
                    selected: i == signal_sel,
                })
                .collect()
        })
        .unwrap_or_default();
    let pending_count = signals.iter().filter(|s| s.status == "pending").count();

    let fixture_rows: Vec<FixtureRow> = s
        .as_ref()
        .map(|st| {
            let mut rows = Vec::new();
            let mut last_date = String::new();
            let mut non_header_idx = 0usize;
            for f in &st.fixtures {
                if let Some(lf) = league_filter {
                    let league_match = st.leagues.iter().any(|l| {
                        l.short == lf
                            && (l.teams.iter().any(|t| t == &f.fixture.home)
                                || l.teams.iter().any(|t| t == &f.fixture.away))
                    });
                    if !league_match {
                        continue;
                    }
                }
                if let Some(tf) = team_filter {
                    if f.fixture.home != tf && f.fixture.away != tf {
                        continue;
                    }
                }
                if f.fixture.date != last_date {
                    rows.push(FixtureRow {
                        date: f.fixture.date.clone(),
                        home: String::new(),
                        away: String::new(),
                        has_market: false,
                        selected: false,
                        is_date_header: true,
                    });
                    last_date = f.fixture.date.clone();
                }
                rows.push(FixtureRow {
                    date: f.fixture.date.clone(),
                    home: f.fixture.home.clone(),
                    away: f.fixture.away.clone(),
                    has_market: f.polymarket.is_some(),
                    selected: non_header_idx == fixture_sel,
                    is_date_header: false,
                });
                non_header_idx += 1;
            }
            rows
        })
        .unwrap_or_default();

    let league_picker_items: Vec<String> = s
        .as_ref()
        .map(|st| st.leagues.iter().map(|l| l.short.clone()).collect())
        .unwrap_or_default();

    let team_picker_items: Vec<String> = s
        .as_ref()
        .map(|st| {
            if let Some(lf) = league_filter {
                st.leagues
                    .iter()
                    .find(|l| l.short == lf)
                    .map(|l| l.teams.clone())
                    .unwrap_or_default()
            } else {
                let mut teams: Vec<String> = st
                    .leagues
                    .iter()
                    .flat_map(|l| l.teams.iter().cloned())
                    .collect();
                teams.sort();
                teams.dedup();
                teams
            }
        })
        .unwrap_or_default();

    SportsView {
        pane,
        strategies,
        signals,
        fixtures: fixture_rows,
        pending_count,
        league_filter: league_filter.map(|s| s.to_string()),
        team_filter: team_filter.map(|s| s.to_string()),
        show_league_picker,
        show_team_picker,
        league_picker_items,
        team_picker_items,
        league_picker_sel,
        team_picker_sel,
    }
}
