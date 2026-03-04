//! Backtester tab: strategy selection → data config → tool params → results + graph.
mod center;
mod data_dialog;
mod graph;
mod help;
mod left;
mod right;

use crate::backtester::BacktesterState;
use crate::tui::router::DataConfigDialog;
use crate::tui::theme::Theme;
use ratatui::{
    layout::{Constraint, Direction, Layout as RLayout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

/// Pane indices for Tab navigation (Results = 4 is view-only, not Tab-navigable).
pub const PANE_STRATEGY: usize = 0;
pub const PANE_DATA: usize = 1;
pub const PANE_TOOL: usize = 2;
pub const PANE_PARAMS: usize = 3;
pub const PANE_RESULTS: usize = 4;
pub const NUM_PANES: usize = 5;

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
    param_sel: usize,
    param_editing: bool,
    param_input: &str,
    show_graph: bool,
    show_help: bool,
    data_dialog: Option<&DataConfigDialog>,
) {
    if area.width < 20 || area.height < 8 {
        let msg = Paragraph::new("Resize terminal (min 20×8)")
            .style(Theme::dim())
            .block(Block::default().borders(Borders::ALL));
        f.render_widget(msg, area);
        return;
    }

    // Main 3-column layout + bottom About bar
    let vert = RLayout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(6), Constraint::Length(3)])
        .split(area);

    let main_area = vert[0];
    let about_area = vert[1];

    let horiz = RLayout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(25),
            Constraint::Percentage(35),
            Constraint::Percentage(40),
        ])
        .split(main_area);

    left::render(
        f,
        horiz[0],
        state,
        focused_pane,
        strategy_sel,
        selected_strategy,
        data_sel,
        tool_sel,
    );

    center::render(
        f,
        horiz[1],
        state,
        tool_sel,
        focused_pane == PANE_PARAMS,
        param_sel,
        param_editing,
        param_input,
    );

    right::render(f, horiz[2], state, focused_pane == PANE_RESULTS);

    render_about(f, about_area, tool_sel);

    // ── Overlays (rendered last so they appear on top) ──────────────────────
    if show_graph {
        render_graph_fullscreen(f, state);
    } else if show_help {
        help::render(f);
    } else if let Some(dialog) = data_dialog {
        data_dialog::render(f, dialog);
    }
}

fn render_about(f: &mut Frame, area: Rect, tool_sel: usize) {
    let tool = crate::backtester::BacktestTool::all()
        .get(tool_sel)
        .copied()
        .unwrap_or(crate::backtester::BacktestTool::HistoricalReplay);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Theme::block_border())
        .title(Span::styled(
            format!(" {} ", tool.name()),
            Theme::block_title(),
        ))
        .style(Style::default().bg(Theme::BG()));

    let max_w = area.width.saturating_sub(4) as usize;
    let tooltip: String = tool.tooltip().chars().take(max_w).collect();

    let para = Paragraph::new(Line::from(Span::styled(tooltip, Theme::dim()))).block(block);
    f.render_widget(para, area);
}

fn render_graph_fullscreen(f: &mut Frame, state: &BacktesterState) {
    let area = f.area();
    // Leave a small margin
    let margin = 2u16;
    let graph_area = Rect {
        x: area.x + margin,
        y: area.y + margin,
        width: area.width.saturating_sub(margin * 2),
        height: area.height.saturating_sub(margin * 2),
    };

    f.render_widget(Clear, graph_area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Theme::tab_selected().add_modifier(Modifier::BOLD))
        .title(Span::styled(
            " Graph — Full Screen  [g or any key to close] ",
            Theme::tab_selected().add_modifier(Modifier::BOLD),
        ))
        .style(Style::default().bg(Theme::BG()));

    // Get domain hint from state
    let domain = get_active_domain(state);
    let inner = block.inner(graph_area);
    f.render_widget(block, graph_area);
    graph::render_in_area(f, inner, state, domain);
}

fn get_active_domain(state: &BacktesterState) -> &'static str {
    // Peek at tool result to determine domain, fallback to "all"
    if let Ok(guard) = state.tool_result.read() {
        if guard.is_some() {
            // Could inspect trades domain in future
        }
    }
    "all"
}
