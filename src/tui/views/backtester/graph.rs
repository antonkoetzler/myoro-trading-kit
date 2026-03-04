//! Graph pane: renders equity curve sparkline, spaghetti plot, or histogram.
use crate::backtester::BacktesterState;
use crate::tui::theme::Theme;
use ratatui::{
    layout::Rect,
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Sparkline},
    Frame,
};

pub fn render(f: &mut Frame, area: Rect, state: &BacktesterState) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Theme::block_border())
        .title(Span::styled(" Graph ", Theme::block_title()));

    let tool_result = state.tool_result.read().ok();

    let tr = match tool_result.as_ref().and_then(|r| r.as_ref()) {
        Some(r) => r,
        None => {
            let para = Paragraph::new("  Run a tool to see the graph.")
                .style(Theme::dim())
                .block(block);
            f.render_widget(para, area);
            return;
        }
    };

    // Priority: histogram > equity curve > text
    if !tr.histogram.is_empty() {
        render_ascii_histogram(f, area, &tr.histogram, block);
    } else if !tr.equity_curve.is_empty() {
        render_equity_sparkline(f, area, &tr.equity_curve, &tr.extra_curves, block);
    } else {
        let para = Paragraph::new("  No graph data.")
            .style(Theme::dim())
            .block(block);
        f.render_widget(para, area);
    }
}

/// Render graph directly into `area` (no outer block) — used by the fullscreen overlay.
pub fn render_in_area(f: &mut Frame, area: Rect, state: &BacktesterState, _domain: &str) {
    if area.height < 3 || area.width < 10 {
        return;
    }
    let tool_result = state.tool_result.read().ok();
    let tr = match tool_result.as_ref().and_then(|r| r.as_ref()) {
        Some(r) => r,
        None => {
            let para = Paragraph::new(vec![
                Line::from(""),
                Line::from(Span::styled(
                    "  Run a tool to see the graph  (press r from any pane).",
                    Theme::dim(),
                )),
            ]);
            f.render_widget(para, area);
            return;
        }
    };

    if !tr.extra_curves.is_empty() || tr.equity_curve.len() > 1 {
        // Spaghetti / permutation plot fills the whole area
        render_spaghetti_ascii(f, area, &tr.equity_curve, &tr.extra_curves);
    } else if !tr.histogram.is_empty() {
        render_histogram_bare(f, area, &tr.histogram);
    } else if !tr.equity_curve.is_empty() {
        render_sparkline_bare(f, area, &tr.equity_curve);
    } else {
        f.render_widget(
            Paragraph::new(Span::styled("  No graph data.", Theme::dim())),
            area,
        );
    }
}

fn render_sparkline_bare(f: &mut Frame, area: Rect, curve: &[f64]) {
    let min = curve.iter().cloned().fold(f64::INFINITY, f64::min);
    let max = curve.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let range = (max - min).max(1.0);
    let w = area.width as usize;
    let step = (curve.len() as f64 / w as f64).ceil() as usize;
    let data: Vec<u64> = curve
        .chunks(step.max(1))
        .map(|chunk| {
            let avg = chunk.iter().sum::<f64>() / chunk.len() as f64;
            ((avg - min) / range * 100.0).round() as u64
        })
        .collect();
    let sparkline = Sparkline::default()
        .data(&data)
        .style(Theme::tab_selected());
    f.render_widget(sparkline, area);
}

fn render_histogram_bare(f: &mut Frame, area: Rect, data: &[(String, u64)]) {
    if data.is_empty() || area.height < 2 {
        return;
    }
    let max_count = data.iter().map(|(_, c)| *c).max().unwrap_or(1).max(1);
    let bar_width = (area.width as usize).saturating_sub(14);
    let lines: Vec<Line> = data
        .iter()
        .take(area.height as usize)
        .map(|(label, count)| {
            let bar_len = (*count as f64 / max_count as f64 * bar_width as f64).round() as usize;
            let bar: String = "█".repeat(bar_len);
            Line::from(vec![
                Span::styled(format!("{:>10} ", label), Theme::dim()),
                Span::styled(bar, Theme::tab_selected()),
                Span::styled(format!(" {}", count), Theme::dim()),
            ])
        })
        .collect();
    f.render_widget(Paragraph::new(lines), area);
}

fn render_equity_sparkline(
    f: &mut Frame,
    area: Rect,
    curve: &[f64],
    extra: &[Vec<f64>],
    block: Block<'_>,
) {
    if curve.is_empty() {
        let para = Paragraph::new("  No data.")
            .style(Theme::dim())
            .block(block);
        f.render_widget(para, area);
        return;
    }

    let inner = block.inner(area);
    f.render_widget(block, area);

    if inner.height < 2 || inner.width < 4 {
        return;
    }

    // If there are extra curves (spaghetti plot), render as ASCII
    if !extra.is_empty() {
        render_spaghetti_ascii(f, inner, curve, extra);
        return;
    }

    // Simple sparkline for single equity curve
    let min = curve.iter().cloned().fold(f64::INFINITY, f64::min);
    let max = curve.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let range = (max - min).max(1.0);

    // Downsample to fit width
    let w = inner.width as usize;
    let step = (curve.len() as f64 / w as f64).ceil() as usize;
    let data: Vec<u64> = curve
        .chunks(step.max(1))
        .map(|chunk| {
            let avg = chunk.iter().sum::<f64>() / chunk.len() as f64;
            ((avg - min) / range * 100.0).round() as u64
        })
        .collect();

    let sparkline = Sparkline::default()
        .data(&data)
        .style(Theme::tab_selected());
    f.render_widget(sparkline, inner);
}

fn render_spaghetti_ascii(f: &mut Frame, area: Rect, real: &[f64], perms: &[Vec<f64>]) {
    let h = area.height as usize;
    let w = area.width as usize;
    if h < 3 || w < 10 {
        return;
    }

    // Find global min/max across all curves
    let mut all_min = f64::INFINITY;
    let mut all_max = f64::NEG_INFINITY;
    for v in real.iter() {
        all_min = all_min.min(*v);
        all_max = all_max.max(*v);
    }
    for curve in perms.iter().take(30) {
        for v in curve {
            all_min = all_min.min(*v);
            all_max = all_max.max(*v);
        }
    }
    let range = (all_max - all_min).max(1.0);

    // Build character grid
    let mut grid = vec![vec![' '; w]; h];
    let step = |curve: &[f64]| -> usize { (curve.len() as f64 / w as f64).ceil() as usize };

    // Draw permuted curves (dim)
    for curve in perms.iter().take(30) {
        let s = step(curve).max(1);
        for (x, chunk) in curve.chunks(s).enumerate().take(w) {
            let avg = chunk.iter().sum::<f64>() / chunk.len() as f64;
            let y = ((all_max - avg) / range * (h - 1) as f64).round() as usize;
            if x < w && y < h {
                grid[y][x] = '·';
            }
        }
    }

    // Draw real curve (bold)
    let s = step(real).max(1);
    for (x, chunk) in real.chunks(s).enumerate().take(w) {
        let avg = chunk.iter().sum::<f64>() / chunk.len() as f64;
        let y = ((all_max - avg) / range * (h - 1) as f64).round() as usize;
        if x < w && y < h {
            grid[y][x] = '█';
        }
    }

    let lines: Vec<Line> = grid
        .iter()
        .map(|row| {
            let s: String = row.iter().collect();
            Line::from(Span::styled(s, Theme::dim()))
        })
        .collect();

    let para = Paragraph::new(lines);
    f.render_widget(para, area);
}

fn render_ascii_histogram(f: &mut Frame, area: Rect, data: &[(String, u64)], block: Block<'_>) {
    let inner = block.inner(area);
    f.render_widget(block, area);

    if inner.height < 3 || inner.width < 10 || data.is_empty() {
        return;
    }

    let max_count = data.iter().map(|(_, c)| *c).max().unwrap_or(1).max(1);
    let bar_width = (inner.width as usize).saturating_sub(12);

    let lines: Vec<Line> = data
        .iter()
        .take(inner.height as usize)
        .map(|(label, count)| {
            let bar_len = (*count as f64 / max_count as f64 * bar_width as f64).round() as usize;
            let bar: String = "█".repeat(bar_len);
            Line::from(vec![
                Span::styled(format!("{:>8} ", label), Theme::dim()),
                Span::styled(bar, Theme::tab_selected()),
                Span::styled(format!(" {}", count), Theme::dim()),
            ])
        })
        .collect();

    let para = Paragraph::new(lines);
    f.render_widget(para, inner);
}
