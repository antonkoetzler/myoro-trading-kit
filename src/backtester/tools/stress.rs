//! Stress Testing and Regime Detection tools.
use super::get_param;
use crate::backtester::data::{equity_curve, ToolParam, Trade};
use crate::backtester::metrics::PerformanceMetrics;
use crate::backtester::tools::ToolResult;
use rand::rngs::SmallRng;
use rand::{Rng, SeedableRng};

pub fn stress_params() -> Vec<ToolParam> {
    vec![
        ToolParam::new("Severity", 2.0, 1.0, 5.0, 0.5),
        ToolParam::new("Scenario", 1.0, 1.0, 4.0, 1.0),
    ]
}

pub fn regime_params() -> Vec<ToolParam> {
    vec![ToolParam::new("Regimes", 3.0, 2.0, 4.0, 1.0)]
}

/// Stress test: replay trades under adverse scenarios.
pub fn run(trades: &[Trade], params: &[ToolParam]) -> ToolResult {
    let severity = get_param(params, "Severity", 2.0);
    let scenario = get_param(params, "Scenario", 1.0) as usize;
    if trades.is_empty() {
        return ToolResult {
            summary: vec![("Error".into(), "No trades.".into())],
            ..Default::default()
        };
    }

    let scenario_name = match scenario {
        1 => "Odds Cliff (50c→5c)",
        2 => "Mass Resolution NO",
        3 => "Liquidity Drain",
        _ => "Volatility Shock",
    };

    let stressed: Vec<Trade> = trades
        .iter()
        .map(|t| {
            let pnl = match scenario {
                1 => {
                    if t.pnl < 0.0 {
                        t.pnl * severity
                    } else {
                        t.pnl * 0.5
                    }
                }
                2 => {
                    if t.pnl < 0.0 {
                        t.pnl * severity * 1.5
                    } else {
                        t.pnl * 0.3
                    }
                }
                3 => t.pnl - t.size * 0.02 * severity,
                _ => {
                    let mut rng = SmallRng::seed_from_u64(t.timestamp as u64);
                    t.pnl * (1.0 + (rng.gen::<f64>() - 0.5) * severity)
                }
            };
            Trade { pnl, ..t.clone() }
        })
        .collect();

    let base = PerformanceMetrics::compute(trades);
    let stress = PerformanceMetrics::compute(&stressed);
    let curve_base = equity_curve(trades);
    let curve_stress = equity_curve(&stressed);

    ToolResult {
        summary: vec![
            ("Scenario".into(), scenario_name.into()),
            ("Severity".into(), format!("{:.1}x", severity)),
            ("Base Return".into(), format!("{:.2}", base.total_return)),
            (
                "Stressed Return".into(),
                format!("{:.2}", stress.total_return),
            ),
            (
                "Base MaxDD".into(),
                format!("{:.2}%", base.max_dd_pct * 100.0),
            ),
            (
                "Stressed MaxDD".into(),
                format!("{:.2}%", stress.max_dd_pct * 100.0),
            ),
            ("Base Sharpe".into(), format!("{:.3}", base.sharpe)),
            ("Stressed Sharpe".into(), format!("{:.3}", stress.sharpe)),
        ],
        equity_curve: curve_base,
        extra_curves: vec![curve_stress],
        detail_lines: vec![
            format!("Scenario: {} at {:.1}x severity", scenario_name, severity),
            format!(
                "Return impact: {:.2} → {:.2} ({:.1}%)",
                base.total_return,
                stress.total_return,
                if base.total_return.abs() > 0.0 {
                    (stress.total_return / base.total_return - 1.0) * 100.0
                } else {
                    0.0
                }
            ),
        ],
        ..Default::default()
    }
}

/// Simple regime detection via rolling volatility clustering.
pub fn regime_detect(trades: &[Trade], params: &[ToolParam]) -> ToolResult {
    let n_regimes = get_param(params, "Regimes", 3.0) as usize;
    if trades.len() < 20 {
        return ToolResult {
            summary: vec![("Error".into(), "Need at least 20 trades.".into())],
            ..Default::default()
        };
    }

    let pnls: Vec<f64> = trades.iter().map(|t| t.pnl).collect();
    let window = (trades.len() / 10).max(5);

    // Compute rolling volatility
    let mut vols: Vec<f64> = Vec::new();
    for i in window..pnls.len() {
        let slice = &pnls[i - window..i];
        let mean = slice.iter().sum::<f64>() / slice.len() as f64;
        let var = slice.iter().map(|p| (p - mean).powi(2)).sum::<f64>() / slice.len() as f64;
        vols.push(var.sqrt());
    }

    // Classify into regimes by vol percentile
    let mut sorted_vols = vols.clone();
    sorted_vols.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let thresholds: Vec<f64> = (1..n_regimes)
        .map(|i| {
            let idx = (i as f64 / n_regimes as f64 * sorted_vols.len() as f64) as usize;
            sorted_vols[idx.min(sorted_vols.len() - 1)]
        })
        .collect();

    let regime_names = ["Low Vol", "Medium Vol", "High Vol", "Extreme Vol"];
    let mut regime_counts = vec![0usize; n_regimes];
    let mut regime_pnls: Vec<Vec<f64>> = vec![Vec::new(); n_regimes];

    for (i, &vol) in vols.iter().enumerate() {
        let regime = thresholds
            .iter()
            .filter(|&&t| vol >= t)
            .count()
            .min(n_regimes - 1);
        regime_counts[regime] += 1;
        if i + window < pnls.len() {
            regime_pnls[regime].push(pnls[i + window]);
        }
    }

    let mut summary = vec![("Regimes".into(), format!("{}", n_regimes))];
    let mut detail = Vec::new();
    for r in 0..n_regimes {
        let name = regime_names.get(r).unwrap_or(&"Unknown");
        let count = regime_counts[r];
        let avg = if regime_pnls[r].is_empty() {
            0.0
        } else {
            regime_pnls[r].iter().sum::<f64>() / regime_pnls[r].len() as f64
        };
        summary.push((format!("{} trades", name), format!("{}", count)));
        summary.push((format!("{} avg P&L", name), format!("{:.2}", avg)));
        detail.push(format!("{}: {} trades, avg P&L {:.2}", name, count, avg));
    }

    ToolResult {
        summary,
        equity_curve: equity_curve(trades),
        detail_lines: detail,
        ..Default::default()
    }
}
