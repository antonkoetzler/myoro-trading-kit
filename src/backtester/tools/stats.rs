//! Statistical tools: Deflated Sharpe Ratio, PBO, Minimum Backtest Length.
use super::get_param;
use crate::backtester::data::{ToolParam, Trade};
use crate::backtester::metrics::PerformanceMetrics;
use crate::backtester::tools::ToolResult;

// ── Deflated Sharpe Ratio ───────────────────────────────────────────────────

pub fn deflated_sharpe_params() -> Vec<ToolParam> {
    vec![ToolParam::new("Strategies Tested", 10.0, 1.0, 200.0, 1.0)]
}

/// Adjusts Sharpe for multiple testing bias (Bailey & Lopez de Prado, 2014).
pub fn deflated_sharpe(trades: &[Trade], params: &[ToolParam]) -> ToolResult {
    let n_trials = get_param(params, "Strategies Tested", 10.0) as usize;
    if trades.len() < 5 {
        return ToolResult {
            summary: vec![("Error".into(), "Need at least 5 trades.".into())],
            ..Default::default()
        };
    }

    let m = PerformanceMetrics::compute(trades);
    let n = trades.len() as f64;
    let pnls: Vec<f64> = trades.iter().map(|t| t.pnl).collect();
    let mean = pnls.iter().sum::<f64>() / n;
    let var = pnls.iter().map(|p| (p - mean).powi(2)).sum::<f64>() / n;
    let skew = pnls
        .iter()
        .map(|p| ((p - mean) / var.sqrt()).powi(3))
        .sum::<f64>()
        / n;
    let kurt = pnls
        .iter()
        .map(|p| ((p - mean) / var.sqrt()).powi(4))
        .sum::<f64>()
        / n
        - 3.0;

    // Expected max Sharpe under null (Euler-Mascheroni)
    let euler_gamma = 0.5772;
    let e_max_sr = (2.0 * (n_trials as f64).ln()).sqrt()
        - (euler_gamma + (2.0 * (n_trials as f64).ln()).ln())
            / (2.0 * (2.0 * (n_trials as f64).ln()).sqrt());

    // Deflated Sharpe
    let sr = m.sharpe / (252.0_f64).sqrt(); // de-annualize
    let se_sr = ((1.0 - skew * sr + (kurt - 1.0) / 4.0 * sr * sr) / n).sqrt();
    let deflated = if se_sr > 0.0 {
        normal_cdf((sr - e_max_sr) / se_sr)
    } else {
        0.0
    };

    ToolResult {
        summary: vec![
            ("Observed Sharpe".into(), format!("{:.4}", m.sharpe)),
            ("Strategies Tested".into(), format!("{}", n_trials)),
            (
                "E[max SR] under null".into(),
                format!("{:.4}", e_max_sr * (252.0_f64).sqrt()),
            ),
            ("Deflated SR prob".into(), format!("{:.4}", deflated)),
            (
                "Significant".into(),
                if deflated > 0.95 { "YES" } else { "NO" }.into(),
            ),
            ("Skewness".into(), format!("{:.3}", skew)),
            ("Excess Kurtosis".into(), format!("{:.3}", kurt)),
        ],
        detail_lines: vec![format!(
            "After testing {} strategies, probability this Sharpe is genuine: {:.1}%",
            n_trials,
            deflated * 100.0
        )],
        ..Default::default()
    }
}

// ── Probability of Backtest Overfitting ──────────────────────────────────────

pub fn pbo_params() -> Vec<ToolParam> {
    vec![ToolParam::new("Subsets", 8.0, 4.0, 16.0, 2.0)]
}

/// Combinatorially symmetric cross-validation. PBO < 0.3 = acceptable.
pub fn pbo(trades: &[Trade], params: &[ToolParam]) -> ToolResult {
    let n_subsets = get_param(params, "Subsets", 8.0) as usize;
    let n = trades.len();
    if n < n_subsets * 2 {
        return ToolResult {
            summary: vec![(
                "Error".into(),
                "Need more trades for this many subsets.".into(),
            )],
            ..Default::default()
        };
    }

    let chunk = n / n_subsets;
    let mut is_sharpes = Vec::new();
    let mut oos_sharpes = Vec::new();

    // Simple CSCV: use each subset as OOS once
    for oos_idx in 0..n_subsets {
        let mut is_trades = Vec::new();
        let mut oos_trades_set = Vec::new();
        for s in 0..n_subsets {
            let start = s * chunk;
            let end = ((s + 1) * chunk).min(n);
            if s == oos_idx {
                oos_trades_set.extend_from_slice(&trades[start..end]);
            } else {
                is_trades.extend_from_slice(&trades[start..end]);
            }
        }
        is_sharpes.push(PerformanceMetrics::compute(&is_trades).sharpe);
        oos_sharpes.push(PerformanceMetrics::compute(&oos_trades_set).sharpe);
    }

    // PBO = fraction of times OOS Sharpe < 0 when IS Sharpe is best
    let overfit_count = oos_sharpes.iter().filter(|&&s| s < 0.0).count();
    let pbo_val = overfit_count as f64 / n_subsets as f64;

    ToolResult {
        summary: vec![
            ("Subsets".into(), format!("{}", n_subsets)),
            ("PBO".into(), format!("{:.3}", pbo_val)),
            (
                "Acceptable".into(),
                if pbo_val < 0.3 {
                    "YES (< 0.3)"
                } else {
                    "NO (≥ 0.3)"
                }
                .into(),
            ),
            ("Avg IS Sharpe".into(), format!("{:.4}", mean(&is_sharpes))),
            (
                "Avg OOS Sharpe".into(),
                format!("{:.4}", mean(&oos_sharpes)),
            ),
        ],
        detail_lines: vec![format!(
            "{}-fold CSCV: {}/{} OOS folds had negative Sharpe.",
            n_subsets, overfit_count, n_subsets
        )],
        ..Default::default()
    }
}

// ── Minimum Backtest Length ──────────────────────────────────────────────────

pub fn min_length_params() -> Vec<ToolParam> {
    vec![
        ToolParam::new("Target Sharpe", 1.5, 0.5, 5.0, 0.1),
        ToolParam::new("Strategies Tested", 10.0, 1.0, 200.0, 1.0),
    ]
}

/// Computes minimum data needed for the Sharpe to be statistically significant.
pub fn min_backtest_length(trades: &[Trade], params: &[ToolParam]) -> ToolResult {
    let target_sr = get_param(params, "Target Sharpe", 1.5);
    let n_trials = get_param(params, "Strategies Tested", 10.0) as usize;

    let pnls: Vec<f64> = trades.iter().map(|t| t.pnl).collect();
    let n = pnls.len() as f64;
    let mean = if n > 0.0 {
        pnls.iter().sum::<f64>() / n
    } else {
        0.0
    };
    let var = if n > 1.0 {
        pnls.iter().map(|p| (p - mean).powi(2)).sum::<f64>() / n
    } else {
        1.0
    };
    let skew = if var > 0.0 {
        pnls.iter()
            .map(|p| ((p - mean) / var.sqrt()).powi(3))
            .sum::<f64>()
            / n
    } else {
        0.0
    };
    let kurt = if var > 0.0 {
        pnls.iter()
            .map(|p| ((p - mean) / var.sqrt()).powi(4))
            .sum::<f64>()
            / n
            - 3.0
    } else {
        0.0
    };

    // MinBTL formula (Bailey & Lopez de Prado): n* = (1 + (1-skew*SR + (kurt-1)/4 * SR^2)) * (z_alpha / SR)^2
    let sr_daily = target_sr / (252.0_f64).sqrt();
    let z = inv_normal(1.0 - 0.05 / (2.0 * n_trials as f64));
    let adj = 1.0 - skew * sr_daily + (kurt - 1.0) / 4.0 * sr_daily * sr_daily;
    let min_n = adj * (z / sr_daily).powi(2);
    let min_months = min_n / 21.0;

    let current_months = n / 21.0;
    let sufficient = current_months >= min_months;

    ToolResult {
        summary: vec![
            ("Target Sharpe".into(), format!("{:.2}", target_sr)),
            ("Strategies Tested".into(), format!("{}", n_trials)),
            ("Min Trades Needed".into(), format!("{:.0}", min_n)),
            ("Min Months".into(), format!("{:.1}", min_months)),
            ("Current Trades".into(), format!("{:.0}", n)),
            (
                "Sufficient".into(),
                if sufficient { "YES" } else { "NO" }.into(),
            ),
        ],
        detail_lines: vec![
            format!(
                "Need {:.0} trades ({:.1} months) for significance.",
                min_n, min_months
            ),
            format!(
                "Currently have {:.0} trades ({:.1} months).",
                n, current_months
            ),
        ],
        ..Default::default()
    }
}

// ── Helpers ─────────────────────────────────────────────────────────────────

fn mean(vals: &[f64]) -> f64 {
    if vals.is_empty() {
        0.0
    } else {
        vals.iter().sum::<f64>() / vals.len() as f64
    }
}

fn normal_cdf(x: f64) -> f64 {
    crate::backtester::math::normal_cdf(x)
}

fn inv_normal(p: f64) -> f64 {
    // Rational approximation (Abramowitz & Stegun 26.2.23)
    if p <= 0.0 {
        return f64::NEG_INFINITY;
    }
    if p >= 1.0 {
        return f64::INFINITY;
    }
    let t = (-2.0 * p.min(1.0 - p).ln()).sqrt();
    let c = [2.515517, 0.802853, 0.010328];
    let d = [1.432788, 0.189269, 0.001308];
    let x =
        t - (c[0] + c[1] * t + c[2] * t * t) / (1.0 + d[0] * t + d[1] * t * t + d[2] * t * t * t);
    if p < 0.5 {
        -x
    } else {
        x
    }
}
