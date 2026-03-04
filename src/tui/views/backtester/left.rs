//! Left column: Strategy list (with domain headers), Data source list (conditional),
//! Tool list — each with scroll and visual selection state.
use crate::backtester::BacktesterState;
use crate::tui::theme::Theme;
use crate::tui::views::header::focused_block;
use ratatui::{
    layout::{Constraint, Direction, Layout as RLayout, Rect},
    style::Modifier,
    text::{Line, Span},
    widgets::{List, ListItem, ListState},
    Frame,
};

use super::{PANE_DATA, PANE_STRATEGY, PANE_TOOL};

/// A visual row — either a non-selectable header or a selectable strategy entry.
enum Row {
    Header(String),
    Item { orig_idx: usize, name: String },
}

#[allow(clippy::too_many_arguments)]
pub fn render(
    f: &mut Frame,
    area: Rect,
    state: &BacktesterState,
    focused_pane: usize,
    strategy_sel: usize,
    selected_strategy: Option<usize>,
    data_sel: usize,
    tool_sel: usize,
) {
    let strategy_selected = selected_strategy.is_some();

    if strategy_selected {
        // Three-pane layout: strategy | data | tools
        let chunks = RLayout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage(30),
                Constraint::Percentage(25),
                Constraint::Percentage(45),
            ])
            .split(area);
        render_strategy_list(
            f,
            chunks[0],
            state,
            focused_pane == PANE_STRATEGY,
            strategy_sel,
            selected_strategy,
        );
        render_data_list(
            f,
            chunks[1],
            state,
            focused_pane == PANE_DATA,
            data_sel,
            selected_strategy,
        );
        render_tool_list(f, chunks[2], focused_pane == PANE_TOOL, tool_sel);
    } else {
        // Two-pane layout: strategy (larger) | tools
        let chunks = RLayout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(45), Constraint::Percentage(55)])
            .split(area);
        render_strategy_list(
            f,
            chunks[0],
            state,
            focused_pane == PANE_STRATEGY,
            strategy_sel,
            selected_strategy,
        );
        // Hint that data source will appear after selection
        let hint_block = focused_block("Data Source", false);
        let hint = ratatui::widgets::Paragraph::new(Line::from(Span::styled(
            "  Select a strategy first (Enter)",
            Theme::dim(),
        )))
        .block(hint_block);
        // We'll use a tiny slice for the hint — just show it dimmed without data sources
        render_tool_list(f, chunks[1], focused_pane == PANE_TOOL, tool_sel);
        let _ = hint; // Data source hint is in the tool pane's header area, skip separate rendering
    }
}

fn render_strategy_list(
    f: &mut Frame,
    area: Rect,
    state: &BacktesterState,
    focused: bool,
    cursor: usize,
    selected: Option<usize>,
) {
    // Build visual rows: "All Strategies" ungrouped, then domain-grouped remainder
    let mut rows: Vec<Row> = Vec::new();
    let mut visual_cursor = 0usize; // visual index of the cursor row

    // "All Strategies" (domain="all") — shown ungrouped at top, no header
    for (orig_idx, s) in state.strategies.iter().enumerate() {
        if s.domain == "all" {
            if orig_idx == cursor {
                visual_cursor = rows.len();
            }
            rows.push(Row::Item {
                orig_idx,
                name: s.name.clone(),
            });
        }
    }

    // Domain-grouped entries
    for (domain_key, domain_label) in [
        ("crypto", "Crypto"),
        ("sports", "Sports"),
        ("weather", "Weather"),
    ] {
        let in_domain: Vec<_> = state
            .strategies
            .iter()
            .enumerate()
            .filter(|(_, s)| s.domain == domain_key)
            .collect();
        if in_domain.is_empty() {
            continue;
        }
        rows.push(Row::Header(format!("── {} ──", domain_label)));
        for (orig_idx, s) in in_domain {
            if orig_idx == cursor {
                visual_cursor = rows.len();
            }
            rows.push(Row::Item {
                orig_idx,
                name: s.name.clone(),
            });
        }
    }

    // Compute scroll offset to keep cursor visible
    let visible_h = area.height.saturating_sub(2) as usize; // subtract block border rows
    let scroll = compute_scroll(visual_cursor, visible_h);

    // Build ListItems from visible slice
    let items: Vec<ListItem> = rows
        .iter()
        .skip(scroll)
        .take(visible_h.max(1))
        .map(|row| match row {
            Row::Header(label) => ListItem::new(Line::from(Span::styled(
                label.clone(),
                Theme::metrics_label(),
            ))),
            Row::Item { orig_idx, name } => {
                let is_cursor = *orig_idx == cursor;
                let is_selected = selected == Some(*orig_idx);
                let prefix = match (is_cursor, is_selected) {
                    (true, true) => "[✓▸] ",
                    (false, true) => "[✓]  ",
                    (true, false) => "[▸]  ",
                    (false, false) => "[ ]  ",
                };
                let style = if is_selected {
                    Theme::success().add_modifier(Modifier::BOLD)
                } else if is_cursor {
                    Theme::tab_selected().add_modifier(Modifier::BOLD)
                } else {
                    Theme::dim()
                };
                ListItem::new(Line::from(Span::styled(
                    format!("{}{}", prefix, name),
                    style,
                )))
            }
        })
        .collect();

    let mut list_state = ListState::default();
    // Highlight the cursor row within the visible slice
    let cursor_in_slice = visual_cursor.saturating_sub(scroll);
    list_state.select(Some(cursor_in_slice));

    let list = List::new(items).block(focused_block("Strategy", focused));
    f.render_stateful_widget(list, area, &mut list_state);
}

fn render_data_list(
    f: &mut Frame,
    area: Rect,
    state: &BacktesterState,
    focused: bool,
    selected: usize,
    active_strategy: Option<usize>,
) {
    // Filter data sources by the selected strategy's domain
    let strategy_domain = active_strategy
        .and_then(|i| state.strategies.get(i))
        .map(|s| s.domain.as_str())
        .unwrap_or("all");

    let compatible: Vec<(usize, &crate::backtester::data::DataSourceEntry)> = state
        .data_sources
        .iter()
        .enumerate()
        .filter(|(_, d)| d.domain == "all" || d.domain == strategy_domain)
        .collect();

    // Remap selected to compatible index (clamp to last if out of range)
    let compat_sel = selected.min(compatible.len().saturating_sub(1));

    let visible_h = area.height.saturating_sub(2) as usize;
    let scroll = compute_scroll(compat_sel, visible_h);

    let items: Vec<ListItem> = compatible
        .iter()
        .skip(scroll)
        .take(visible_h.max(1))
        .enumerate()
        .map(|(vis_i, (_, d))| {
            let real_i = vis_i + scroll;
            let sel = real_i == compat_sel;
            let prefix = if sel { "[▸] " } else { "[ ] " };
            let style = if sel {
                Theme::tab_selected().add_modifier(Modifier::BOLD)
            } else {
                Theme::dim()
            };
            ListItem::new(Line::from(vec![
                Span::styled(prefix, style),
                Span::styled(d.name.clone(), style),
                Span::styled(" [Enter to configure]", Theme::dim()),
            ]))
        })
        .collect();

    let mut list_state = ListState::default();
    list_state.select(Some(compat_sel.saturating_sub(scroll)));

    let list = List::new(items).block(focused_block("Data Source", focused));
    f.render_stateful_widget(list, area, &mut list_state);
}

fn render_tool_list(f: &mut Frame, area: Rect, focused: bool, selected: usize) {
    let tools = crate::backtester::BacktestTool::all();
    let visible_h = area.height.saturating_sub(2) as usize;
    let scroll = compute_scroll(selected, visible_h);

    let items: Vec<ListItem> = tools
        .iter()
        .enumerate()
        .skip(scroll)
        .take(visible_h.max(1))
        .map(|(i, t)| {
            let sel = i == selected;
            let prefix = if sel { "[●] " } else { "[ ] " };
            let style = if sel {
                Theme::tab_selected().add_modifier(Modifier::BOLD)
            } else {
                Theme::dim()
            };
            ListItem::new(Line::from(Span::styled(
                format!("{}{}", prefix, t.name()),
                style,
            )))
        })
        .collect();

    let mut list_state = ListState::default();
    list_state.select(Some(selected.saturating_sub(scroll)));

    let list = List::new(items).block(focused_block("Tools", focused));
    f.render_stateful_widget(list, area, &mut list_state);
}

/// Compute scroll offset to keep `cursor` visible in `visible_h` rows.
/// Centers the cursor in the view once it goes out of the visible window.
fn compute_scroll(cursor: usize, visible_h: usize) -> usize {
    if visible_h == 0 || cursor < visible_h {
        0
    } else {
        cursor.saturating_sub(visible_h / 2)
    }
}
