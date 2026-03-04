//! Center column: tool parameters (editable).
use crate::backtester::BacktesterState;
use crate::tui::theme::Theme;
use crate::tui::views::header::focused_block;
use ratatui::{
    layout::Rect,
    style::Modifier,
    text::{Line, Span},
    widgets::{Paragraph, Wrap},
    Frame,
};

#[allow(clippy::too_many_arguments)]
pub fn render(
    f: &mut Frame,
    area: Rect,
    state: &BacktesterState,
    tool_sel: usize,
    focused: bool,
    param_sel: usize,
    param_editing: bool,
    param_input: &str,
) {
    let tool = crate::backtester::BacktestTool::all()
        .get(tool_sel)
        .copied()
        .unwrap_or(crate::backtester::BacktestTool::HistoricalReplay);

    let title = format!("{} — Params", tool.name());
    let block = focused_block(&title, focused);

    let params = state
        .tool_params
        .read()
        .ok()
        .and_then(|p| p.get(tool_sel).cloned())
        .unwrap_or_default();

    if params.is_empty() {
        let lines = vec![
            Line::from(""),
            Line::from(Span::styled("  No adjustable parameters.", Theme::dim())),
            Line::from(""),
            Line::from(Span::styled("  Press r to run analysis.", Theme::dim())),
        ];
        let para = Paragraph::new(lines).block(block).wrap(Wrap { trim: true });
        f.render_widget(para, area);
        return;
    }

    let mut lines = Vec::new();
    for (i, p) in params.iter().enumerate() {
        let is_sel = i == param_sel && focused;
        let is_editing = is_sel && param_editing;

        let prefix = if is_sel { "▸ " } else { "  " };

        let name_style = if is_sel {
            Theme::tab_selected().add_modifier(Modifier::BOLD)
        } else {
            Theme::dim()
        };

        let value_str = if is_editing {
            format!("[{}▏]", param_input)
        } else {
            format_value(p.value, p.step)
        };

        let value_style = if is_editing {
            Theme::body().add_modifier(Modifier::BOLD)
        } else if is_sel {
            Theme::tab_selected()
        } else {
            Theme::body()
        };

        lines.push(Line::from(vec![
            Span::styled(prefix.to_string(), name_style),
            Span::styled(format!("{:<20}", p.name), name_style),
            Span::styled(value_str, value_style),
        ]));

        // Show range hint for selected param
        if is_sel && !is_editing {
            lines.push(Line::from(Span::styled(
                format!(
                    "    range: {} – {}  step: {}",
                    format_value(p.min, p.step),
                    format_value(p.max, p.step),
                    format_value(p.step, p.step)
                ),
                Theme::dim(),
            )));
        }
    }

    lines.push(Line::from(""));
    if focused {
        lines.push(Line::from(Span::styled(
            "  ←/→ adjust  Enter edit  r run",
            Theme::dim(),
        )));
    }

    let para = Paragraph::new(lines).block(block).wrap(Wrap { trim: true });
    f.render_widget(para, area);
}

fn format_value(val: f64, step: f64) -> String {
    if step >= 1.0 {
        format!("{:.0}", val)
    } else if step >= 0.1 {
        format!("{:.1}", val)
    } else {
        format!("{:.2}", val)
    }
}
