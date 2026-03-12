//! Sensitivity Analysis: sweep strategy parameters, show metric heatmap.
use super::get_param;
use crate::backtester::data::{ToolParam, Trade};
use crate::backtester::metrics::PerformanceMetrics;
use crate::backtester::tools::ToolResult;

pub fn default_params() -> Vec<ToolParam> {
    vec![
        ToolParam::new("Grid Size", 10.0, 5.0, 50.0, 5.0),
        ToolParam::new("Win Rate Sweep", 0.10, 0.05, 0.30, 0.05),
        ToolParam::new("Size Sweep", 0.50, 0.10, 2.0, 0.10),
    ]
}

/// Sweep win-rate and size multipliers around the base strategy, reporting Sharpe.
pub fn analyze(trades: &[Trade], params: &[ToolParam]) -> ToolResult {
    let grid = get_param(params, "Grid Size", 10.0) as usize;
    let wr_sweep = get_param(params, "Win Rate Sweep", 0.10);
    let sz_sweep = get_param(params, "Size Sweep", 0.50);

    if trades.len() < 5 {
        return ToolResult {
            summary: vec![("Error".into(), "Need at least 5 trades.".into())],
            ..Default::default()
        };
    }

    let base = PerformanceMetrics::compute(trades);
    let base_wr = base.win_rate;
    let pnls: Vec<f64> = trades.iter().map(|t| t.pnl).collect();

    let mut detail = Vec::new();
    let mut best_sharpe = f64::NEG_INFINITY;
    let mut best_params = String::new();
    let mut worst_sharpe = f64::INFINITY;
    // grid_sharpes[wr_step * grid + sz_step]
    let mut grid_sharpes: Vec<f64> = Vec::with_capacity(grid * grid);

    for wr_step in 0..grid {
        let wr_offset = -wr_sweep + 2.0 * wr_sweep * wr_step as f64 / (grid - 1).max(1) as f64;
        let adj_wr = (base_wr + wr_offset).clamp(0.05, 0.95);

        for sz_step in 0..grid {
            let sz_mult =
                1.0 - sz_sweep + 2.0 * sz_sweep * sz_step as f64 / (grid - 1).max(1) as f64;
            let adjusted: Vec<Trade> = trades
                .iter()
                .enumerate()
                .map(|(i, t)| {
                    let is_win = (i as f64 / trades.len() as f64) < adj_wr;
                    let pnl = if is_win {
                        pnls[i].abs() * sz_mult
                    } else {
                        -pnls[i].abs() * sz_mult
                    };
                    Trade {
                        pnl,
                        size: t.size * sz_mult,
                        ..t.clone()
                    }
                })
                .collect();
            let m = PerformanceMetrics::compute(&adjusted);
            grid_sharpes.push(m.sharpe);

            if m.sharpe > best_sharpe {
                best_sharpe = m.sharpe;
                best_params = format!("WR={:.2} Size={:.2}x", adj_wr, sz_mult);
            }
            if m.sharpe < worst_sharpe {
                worst_sharpe = m.sharpe;
            }
        }
    }

    let plateau = (best_sharpe - worst_sharpe).abs() < best_sharpe.abs() * 0.3;
    detail.push(format!(
        "Grid: {}×{} = {} combinations",
        grid,
        grid,
        grid * grid
    ));
    detail.push(format!("Best: {} → Sharpe {:.4}", best_params, best_sharpe));
    detail.push(format!(
        "Range: {:.4} to {:.4} ({})",
        worst_sharpe,
        best_sharpe,
        if plateau {
            "PLATEAU — robust"
        } else {
            "SPIKY — potential overfit"
        }
    ));

    // ASCII heatmap: rows = WinRate sweep, cols = Size sweep
    // Block chars encode Sharpe quartile: ░▒▓█ (low → high)
    detail.push(String::new());
    detail.push("Heatmap — WinRate(row)×Size(col): ░low ▒med ▓high █peak".to_string());
    let sh_min = grid_sharpes.iter().cloned().fold(f64::INFINITY, f64::min);
    let sh_max = grid_sharpes
        .iter()
        .cloned()
        .fold(f64::NEG_INFINITY, f64::max);
    let sh_range = (sh_max - sh_min).max(0.001);
    const BLOCKS: [char; 4] = ['░', '▒', '▓', '█'];
    for wr_step in 0..grid {
        let wr_offset = -wr_sweep + 2.0 * wr_sweep * wr_step as f64 / (grid - 1).max(1) as f64;
        let adj_wr = (base_wr + wr_offset).clamp(0.05, 0.95);
        let row_str: String = (0..grid)
            .map(|sz_step| {
                let v = grid_sharpes
                    .get(wr_step * grid + sz_step)
                    .copied()
                    .unwrap_or(sh_min);
                let idx = ((v - sh_min) / sh_range * 3.0).round() as usize;
                BLOCKS[idx.min(3)]
            })
            .collect();
        detail.push(format!(" WR{:.2} {}", adj_wr, row_str));
    }

    // Build histogram of grid sharpes
    let histogram = build_sharpe_hist(&grid_sharpes, 10);

    ToolResult {
        summary: vec![
            ("Grid".into(), format!("{}×{}", grid, grid)),
            ("Base Sharpe".into(), format!("{:.4}", base.sharpe)),
            ("Best Sharpe".into(), format!("{:.4}", best_sharpe)),
            ("Worst Sharpe".into(), format!("{:.4}", worst_sharpe)),
            (
                "Robust".into(),
                if plateau {
                    "YES (plateau)"
                } else {
                    "NO (spiky)"
                }
                .into(),
            ),
        ],
        histogram,
        detail_lines: detail,
        ..Default::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backtester::data::Trade;
    use crate::shared::strategy::Side;

    fn make_trades(n: usize) -> Vec<Trade> {
        (0..n)
            .map(|i| Trade {
                timestamp: 0,
                side: Side::Yes,
                entry_price: 0.45,
                exit_price: 0.55,
                size: 10.0,
                pnl: if i % 2 == 0 { 5.0 } else { -3.0 },
                strategy_id: "test".to_string(),
            })
            .collect()
    }

    #[test]
    fn sensitivity_heatmap_in_detail_lines() {
        let trades = make_trades(20);
        let params = default_params();
        let result = analyze(&trades, &params);
        let heatmap_line = result.detail_lines.iter().find(|l| l.contains("Heatmap"));
        assert!(
            heatmap_line.is_some(),
            "detail_lines should contain Heatmap header"
        );
    }

    #[test]
    fn sensitivity_heatmap_uses_block_chars() {
        let trades = make_trades(20);
        let params = default_params();
        let result = analyze(&trades, &params);
        const BLOCKS: [char; 4] = ['░', '▒', '▓', '█'];
        let heatmap_rows: Vec<&String> = result
            .detail_lines
            .iter()
            .filter(|l| l.trim_start().starts_with("WR"))
            .collect();
        assert!(
            !heatmap_rows.is_empty(),
            "should have WR rows in detail_lines"
        );
        for row in &heatmap_rows {
            for ch in row.chars() {
                if BLOCKS.contains(&ch) {
                    return; // found at least one block char
                }
            }
        }
        panic!("No block chars found in heatmap rows");
    }

    #[test]
    fn sensitivity_returns_error_for_few_trades() {
        let result = analyze(&[], &default_params());
        let has_error = result.summary.iter().any(|(k, _)| k == "Error");
        assert!(has_error);
    }
}

fn build_sharpe_hist(vals: &[f64], bins: usize) -> Vec<(String, u64)> {
    if vals.is_empty() {
        return Vec::new();
    }
    let min = vals.iter().cloned().fold(f64::INFINITY, f64::min);
    let max = vals.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let range = (max - min).max(0.001);
    let bw = range / bins as f64;
    let mut counts = vec![0u64; bins];
    for &v in vals {
        let idx = ((v - min) / bw).floor() as usize;
        counts[idx.min(bins - 1)] += 1;
    }
    counts
        .into_iter()
        .enumerate()
        .map(|(i, c)| (format!("{:.2}", min + i as f64 * bw), c))
        .collect()
}
