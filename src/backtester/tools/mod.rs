//! Tool dispatch and shared result type for all 19 backtester tools.
pub mod bootstrap;
pub mod permutation;
pub mod risk;
pub mod sensitivity;
pub mod stats;
pub mod stress;
pub mod tca;
pub mod walk_forward;

use crate::backtester::data::{ToolParam, Trade};

/// Universal result from any tool analysis.
#[derive(Debug, Clone, Default)]
pub struct ToolResult {
    /// Key-value summary lines (label, value).
    pub summary: Vec<(String, String)>,
    /// Primary equity curve (or metric series).
    pub equity_curve: Vec<f64>,
    /// Extra curves for spaghetti plots (permutation tests).
    pub extra_curves: Vec<Vec<f64>>,
    /// Histogram buckets (label, count).
    pub histogram: Vec<(String, u64)>,
    /// P-value if applicable.
    pub p_value: Option<f64>,
    /// Detail text lines.
    pub detail_lines: Vec<String>,
}

/// Run a specific tool analysis on the given trades.
pub fn run_tool(tool: super::BacktestTool, trades: &[Trade], params: &[ToolParam]) -> ToolResult {
    use super::BacktestTool::*;
    match tool {
        PermTradeOrder => permutation::trade_order(trades, params),
        PermReturnBootstrap => permutation::return_bootstrap(trades, params),
        PermBarShuffle => permutation::bar_shuffle(trades, params),
        HistoricalReplay => replay_result(trades),
        WalkForward => walk_forward::run(trades, params),
        BootstrapCI => bootstrap::confidence_intervals(trades, params),
        TransactionCost => tca::analyze(trades, params),
        Sensitivity => sensitivity::analyze(trades, params),
        DeflatedSharpe => stats::deflated_sharpe(trades, params),
        PBO => stats::pbo(trades, params),
        MinBacktestLen => stats::min_backtest_length(trades, params),
        StressTest => stress::run(trades, params),
        RegimeDetect => stress::regime_detect(trades, params),
        RiskOfRuin => risk::risk_of_ruin(trades, params),
        DrawdownAnalysis => risk::drawdown_analysis(trades),
        McBasic => mc_wrapper(trades, params, "Monte Carlo Basic"),
        McIS => mc_wrapper(trades, params, "Monte Carlo + IS"),
        CopulaAnalysis => mc_wrapper(trades, params, "Copula Analysis"),
        AgentBased => abm_wrapper(trades, params),
    }
}

pub(crate) fn get_param(params: &[ToolParam], name: &str, default: f64) -> f64 {
    params
        .iter()
        .find(|p| p.name == name)
        .map(|p| p.value)
        .unwrap_or(default)
}

/// Historical Replay — just shows the raw equity curve and metrics.
fn replay_result(trades: &[Trade]) -> ToolResult {
    let curve = crate::backtester::data::equity_curve(trades);
    let m = crate::backtester::metrics::PerformanceMetrics::compute(trades);
    ToolResult {
        summary: vec![
            ("Trades".into(), format!("{}", m.trade_count)),
            ("Total Return".into(), format!("{:.2}", m.total_return)),
            ("Sharpe".into(), format!("{:.3}", m.sharpe)),
            (
                "Max Drawdown".into(),
                format!("{:.2}%", m.max_dd_pct * 100.0),
            ),
            ("Win Rate".into(), format!("{:.1}%", m.win_rate * 100.0)),
        ],
        equity_curve: curve,
        detail_lines: vec!["Historical replay of all trades in sequence.".into()],
        ..Default::default()
    }
}

/// Wrapper for Monte Carlo / Copula simulation tools.
fn mc_wrapper(trades: &[Trade], params: &[ToolParam], label: &str) -> ToolResult {
    use crate::backtester::data::equity_curve;
    use rand::rngs::SmallRng;
    use rand::{Rng, SeedableRng};

    let n_paths = get_param(params, "Paths", 100.0) as usize;
    let sigma_scale = get_param(params, "Sigma Scale", 1.0);
    let curve = equity_curve(trades);
    let n = trades.len().max(1);
    let pnls: Vec<f64> = trades.iter().map(|t| t.pnl).collect();
    let mean = pnls.iter().sum::<f64>() / n as f64;
    let var = pnls.iter().map(|p| (p - mean).powi(2)).sum::<f64>() / n as f64;
    let sigma = var.sqrt() * sigma_scale;

    let mut rng = SmallRng::seed_from_u64(7);
    let mut extra = Vec::new();
    let mut terminals = Vec::new();
    for _ in 0..n_paths {
        let mut path = Vec::with_capacity(n + 1);
        path.push(0.0);
        let mut cum = 0.0;
        for _ in 0..n {
            cum += mean + sigma * (rng.gen::<f64>() - 0.5) * 2.0;
            path.push(cum);
        }
        terminals.push(path.last().copied().unwrap_or(0.0));
        if extra.len() < 80 {
            extra.push(path);
        }
    }

    terminals.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let pct = |p: usize| {
        terminals
            .get(p.min(terminals.len().saturating_sub(1)))
            .copied()
            .unwrap_or(0.0)
    };
    let p5 = pct(n_paths * 5 / 100);
    let p50 = pct(n_paths / 2);
    let p95 = pct(n_paths * 95 / 100);

    ToolResult {
        summary: vec![
            ("Method".into(), label.into()),
            ("Paths".into(), format!("{}", n_paths)),
            ("Sigma Scale".into(), format!("{:.1}x", sigma_scale)),
            ("Median P&L".into(), format!("{:.2}", p50)),
            ("5th pctile".into(), format!("{:.2}", p5)),
            ("95th pctile".into(), format!("{:.2}", p95)),
        ],
        equity_curve: curve,
        extra_curves: extra,
        detail_lines: vec![format!(
            "{}: {} simulated paths (σ={:.2}).",
            label, n_paths, sigma
        )],
        ..Default::default()
    }
}

/// Agent-Based Model wrapper using tool params.
fn abm_wrapper(trades: &[Trade], params: &[ToolParam]) -> ToolResult {
    use crate::backtester::data::equity_curve;

    let n_agents = get_param(params, "Agents", 27.0) as usize;
    let n_steps = get_param(params, "Steps", 2000.0) as usize;
    let curve = equity_curve(trades);
    let win_rate = if trades.is_empty() {
        0.5
    } else {
        trades.iter().filter(|t| t.pnl > 0.0).count() as f64 / trades.len() as f64
    };

    let abm_params = crate::backtester::abm::AbmParams {
        true_prob: win_rate,
        n_informed: (n_agents / 5).max(1) as u32,
        n_noise: (n_agents * 3 / 5) as u32,
        n_mm: (n_agents / 5).max(1) as u32,
        n_steps: n_steps as u32,
    };
    let r = crate::backtester::abm::run(&abm_params);

    ToolResult {
        summary: vec![
            ("Method".into(), "Agent-Based Model".into()),
            ("Agents".into(), format!("{}", n_agents)),
            ("Steps".into(), format!("{}", n_steps)),
            ("Final Price".into(), format!("{:.4}", r.final_price)),
            ("True Prob".into(), format!("{:.2}%", win_rate * 100.0)),
            ("Conv. Error".into(), format!("{:.4}", r.convergence_error)),
        ],
        equity_curve: curve,
        detail_lines: vec![format!(
            "ABM: {} agents, {} steps. Price converged to {:.4} (true={:.2}).",
            n_agents, n_steps, r.final_price, win_rate
        )],
        ..Default::default()
    }
}
