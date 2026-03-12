//! Backtester: full quant-desk toolbox with 19 analysis tools.
//! Select strategy → data source → tool → adjust params → run → see metrics + graph.
// NOTE: Exceeds 300-line limit — 19 tools + BacktesterState + dispatch must cohere in one type file.
pub mod abm;
pub mod calibration;
pub mod copula;
pub mod data;
pub mod importance_sampling;
pub mod math;
pub mod metrics;
pub mod monte_carlo;
pub mod particle_filter;
pub mod tools;

use calibration::CalibResult;
use data::{DataSourceEntry, StrategyEntry, ToolParam, Trade};
use metrics::PerformanceMetrics;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, RwLock,
};
use tools::ToolResult;

/// All 19 backtester tools organized by tier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BacktestTool {
    // Tier 0 — Permutation Testing
    PermTradeOrder,
    PermReturnBootstrap,
    PermBarShuffle,
    // Tier 1 — Core Backtesting
    HistoricalReplay,
    WalkForward,
    BootstrapCI,
    TransactionCost,
    // Tier 2 — Robustness & Overfitting
    Sensitivity,
    DeflatedSharpe,
    PBO,
    MinBacktestLen,
    // Tier 3 — Risk & Stress
    StressTest,
    RegimeDetect,
    RiskOfRuin,
    DrawdownAnalysis,
    // Tier 4 — Simulation (legacy, reworked)
    McBasic,
    McIS,
    CopulaAnalysis,
    AgentBased,
}

impl BacktestTool {
    pub fn all() -> &'static [BacktestTool] {
        &[
            Self::PermTradeOrder,
            Self::PermReturnBootstrap,
            Self::PermBarShuffle,
            Self::HistoricalReplay,
            Self::WalkForward,
            Self::BootstrapCI,
            Self::TransactionCost,
            Self::Sensitivity,
            Self::DeflatedSharpe,
            Self::PBO,
            Self::MinBacktestLen,
            Self::StressTest,
            Self::RegimeDetect,
            Self::RiskOfRuin,
            Self::DrawdownAnalysis,
            Self::McBasic,
            Self::McIS,
            Self::CopulaAnalysis,
            Self::AgentBased,
        ]
    }

    pub fn name(&self) -> &'static str {
        match self {
            Self::PermTradeOrder => "Perm: Trade Order",
            Self::PermReturnBootstrap => "Perm: Return Bootstrap",
            Self::PermBarShuffle => "Perm: Bar Shuffle",
            Self::HistoricalReplay => "Historical Replay",
            Self::WalkForward => "Walk-Forward Analysis",
            Self::BootstrapCI => "Bootstrap CI",
            Self::TransactionCost => "Transaction Cost",
            Self::Sensitivity => "Sensitivity Analysis",
            Self::DeflatedSharpe => "Deflated Sharpe",
            Self::PBO => "Prob. of Overfit (PBO)",
            Self::MinBacktestLen => "Min Backtest Length",
            Self::StressTest => "Stress Test",
            Self::RegimeDetect => "Regime Detection",
            Self::RiskOfRuin => "Risk of Ruin",
            Self::DrawdownAnalysis => "Drawdown Analysis",
            Self::McBasic => "Monte Carlo Basic",
            Self::McIS => "Monte Carlo + IS",
            Self::CopulaAnalysis => "Copula Analysis",
            Self::AgentBased => "Agent-Based Model",
        }
    }

    pub fn tooltip(&self) -> &'static str {
        match self {
            Self::PermTradeOrder => "Shuffles trade ORDER N times, replots equity curves. If real \
                curve is top 5%, strategy is statistically significant. Params: permutations, confidence.",
            Self::PermReturnBootstrap => "Resamples trade returns WITH replacement. Tests if overall \
                profitability is robust to luck. Params: permutations, confidence.",
            Self::PermBarShuffle => "Shuffles price-series returns, rebuilds synthetic prices, re-runs \
                strategy. Most rigorous — tests pattern detection vs noise. Params: permutations, confidence.",
            Self::HistoricalReplay => "Replays strategy against historical data trade by trade. The \
                baseline backtest showing raw equity curve and metrics.",
            Self::WalkForward => "Optimizes on in-sample, tests on out-of-sample, rolls forward. \
                WFE > 0.5 = robust. Params: IS window %, number of windows.",
            Self::BootstrapCI => "Resamples trade returns to get confidence intervals on every metric. \
                'Sharpe = 2.1 [95% CI: 1.4–2.8]'. Params: iterations, confidence %.",
            Self::TransactionCost => "Models spread + slippage + commission. Shows gross→net return \
                erosion. Params: commission %, slippage bps, spread bps.",
            Self::Sensitivity => "Sweeps strategy parameters across a grid, shows metric heatmap. \
                Plateau = robust, spike = overfit. Params: grid size, sweep range.",
            Self::DeflatedSharpe => "Adjusts Sharpe for number of strategies tested and higher moments. \
                Answers: 'After testing N strategies, is my Sharpe genuine?' Params: strategies tested.",
            Self::PBO => "Combinatorially symmetric cross-validation. PBO < 0.3 = acceptable overfitting \
                level. Params: number of subsets.",
            Self::MinBacktestLen => "Given target Sharpe and trials, computes minimum data needed. \
                'You need 18 months of data for significance.' Params: target Sharpe, trials.",
            Self::StressTest => "Replays trades under adverse scenarios: odds cliff, mass resolution, \
                liquidity drain, vol shock. Params: severity, scenario.",
            Self::RegimeDetect => "Identifies volatility regimes (low/medium/high) and shows strategy \
                metrics per regime. Params: number of regimes.",
            Self::RiskOfRuin => "Given win rate and position sizing, calculates probability of losing X% \
                of capital. Critical for binary outcomes. Params: ruin threshold, position size.",
            Self::DrawdownAnalysis => "Underwater equity curve showing every drawdown's depth, duration, \
                and recovery time.",
            Self::McBasic => "Generates N synthetic paths from trade return distribution, plots P&L \
                distribution fan chart. Params: paths.",
            Self::McIS => "Tilts toward crash scenarios, shows strategy tail risk under rare events. \
                Params: paths.",
            Self::CopulaAnalysis => "Tests strategy across correlated multi-market scenarios using Gaussian \
                and Student-t joint probability modeling. Params: paths.",
            Self::AgentBased => "Simulates strategy as one agent among noise traders + market makers. \
                Tests market impact and price discovery. Params: agents, steps.",
        }
    }

    pub fn default_params(&self) -> Vec<ToolParam> {
        match self {
            Self::PermTradeOrder | Self::PermReturnBootstrap | Self::PermBarShuffle => {
                tools::permutation::default_params()
            }
            Self::HistoricalReplay => vec![],
            Self::WalkForward => tools::walk_forward::default_params(),
            Self::BootstrapCI => tools::bootstrap::default_params(),
            Self::TransactionCost => tools::tca::default_params(),
            Self::Sensitivity => tools::sensitivity::default_params(),
            Self::DeflatedSharpe => tools::stats::deflated_sharpe_params(),
            Self::PBO => tools::stats::pbo_params(),
            Self::MinBacktestLen => tools::stats::min_length_params(),
            Self::StressTest => tools::stress::stress_params(),
            Self::RegimeDetect => tools::stress::regime_params(),
            Self::RiskOfRuin => tools::risk::ruin_params(),
            Self::DrawdownAnalysis => vec![],
            Self::McBasic | Self::McIS | Self::CopulaAnalysis => {
                vec![
                    ToolParam::new("Paths", 100.0, 10.0, 1000.0, 10.0),
                    ToolParam::new("Sigma Scale", 1.0, 0.1, 3.0, 0.1),
                ]
            }
            Self::AgentBased => vec![
                ToolParam::new("Agents", 27.0, 5.0, 100.0, 1.0),
                ToolParam::new("Steps", 2000.0, 100.0, 10000.0, 100.0),
            ],
        }
    }
}

/// Central backtester state — shared across TUI threads.
pub struct BacktesterState {
    /// Available strategies.
    pub strategies: Vec<StrategyEntry>,
    /// Available data sources.
    pub data_sources: Vec<DataSourceEntry>,
    /// Tool parameters per tool (indexed by BacktestTool::all() position).
    pub tool_params: RwLock<Vec<Vec<ToolParam>>>,
    /// Currently loaded trades.
    pub trades: RwLock<Vec<Trade>>,
    /// Computed performance metrics from current trades.
    pub current_metrics: RwLock<Option<PerformanceMetrics>>,
    /// Result from most recent tool run.
    pub tool_result: RwLock<Option<ToolResult>>,
    /// Legacy results (from run_all).
    pub results: RwLock<Vec<BacktestResult>>,
    /// Calibration from paper trades.
    pub calib: RwLock<CalibResult>,
    /// Whether a backtest is currently running.
    pub is_running: AtomicBool,
}

pub struct BacktestResult {
    pub tool: BacktestTool,
    pub probability: f64,
    pub std_error: f64,
    pub ci_lower: f64,
    pub ci_upper: f64,
    pub extra: Vec<(String, String)>,
    pub log: Vec<String>,
    pub computed_at: chrono::DateTime<chrono::Utc>,
}

impl BacktesterState {
    pub fn new() -> Arc<Self> {
        let all_params: Vec<Vec<ToolParam>> = BacktestTool::all()
            .iter()
            .map(|t| t.default_params())
            .collect();
        Arc::new(Self {
            strategies: data::load_strategies_with_registry(),
            data_sources: data::default_data_sources(),
            tool_params: RwLock::new(all_params),
            trades: RwLock::new(Vec::new()),
            current_metrics: RwLock::new(None),
            tool_result: RwLock::new(None),
            results: RwLock::new(Vec::new()),
            calib: RwLock::new(CalibResult {
                brier_score: None,
                n_resolved: 0,
                n_pending: 0,
                accuracy: None,
            }),
            is_running: AtomicBool::new(false),
        })
    }

    /// Load synthetic trades with explicit parameters (from dialog dropdowns).
    pub fn load_synthetic(&self, n: usize, win_rate: f64, avg_win: f64, avg_loss: f64, seed: u64) {
        let trades = data::generate_synthetic_seeded(n, win_rate, avg_win, avg_loss, seed);
        let metrics = PerformanceMetrics::compute(&trades);
        if let Ok(mut guard) = self.trades.write() {
            *guard = trades;
        }
        if let Ok(mut guard) = self.current_metrics.write() {
            *guard = Some(metrics);
        }
    }

    /// Load trades based on selected strategy and data source.
    pub fn load_trades(&self, strategy_idx: usize, data_source_idx: usize) {
        let strategy_id = self
            .strategies
            .get(strategy_idx)
            .map(|s| s.id.as_str())
            .unwrap_or("all");
        let source_id = self
            .data_sources
            .get(data_source_idx)
            .map(|s| s.id.as_str())
            .unwrap_or("paper");

        let trades = match source_id {
            "paper" => data::load_paper_trades("data/paper_trades.jsonl", strategy_id),
            "synthetic" => data::generate_synthetic(200, 0.55, 8.0, 6.0),
            _ => {
                // Try data provider — fetch history, convert price series to trades
                load_from_provider(source_id, strategy_id)
            }
        };

        // If no trades found, generate synthetic as fallback
        let trades = if trades.is_empty() {
            data::generate_synthetic(200, 0.55, 8.0, 6.0)
        } else {
            trades
        };

        let metrics = PerformanceMetrics::compute(&trades);
        if let Ok(mut guard) = self.trades.write() {
            *guard = trades;
        }
        if let Ok(mut guard) = self.current_metrics.write() {
            *guard = Some(metrics);
        }
    }

    /// Run the selected tool on currently loaded trades.
    pub fn run_tool(&self, tool_idx: usize) {
        if self.is_running.swap(true, Ordering::SeqCst) {
            return;
        }

        let tool = BacktestTool::all()
            .get(tool_idx)
            .copied()
            .unwrap_or(BacktestTool::HistoricalReplay);

        let params = self
            .tool_params
            .read()
            .ok()
            .and_then(|p| p.get(tool_idx).cloned())
            .unwrap_or_default();

        let trades = self
            .trades
            .read()
            .ok()
            .map(|t| t.clone())
            .unwrap_or_default();
        let result = tools::run_tool(tool, &trades, &params);

        if let Ok(mut guard) = self.tool_result.write() {
            *guard = Some(result);
        }

        self.is_running.store(false, Ordering::SeqCst);
    }

    /// Legacy: run all legacy simulation tools. Kept for backward compat.
    pub fn run_all(&self, paper_trades_file: &str) {
        if self.is_running.swap(true, Ordering::SeqCst) {
            return;
        }
        let mut results = Vec::with_capacity(19);
        for &tool in BacktestTool::all() {
            results.push(run_legacy_tool(tool));
        }
        if let Ok(mut guard) = self.results.write() {
            *guard = results;
        }
        let records = calibration::load_from_jsonl(paper_trades_file);
        let calib = CalibResult::compute(&records);
        if let Ok(mut guard) = self.calib.write() {
            *guard = calib;
        }
        self.is_running.store(false, Ordering::SeqCst);
    }
}

// ── Convenience accessors for the GUI layer ─────────────────────────────────
impl BacktesterState {
    pub fn strategy_count(&self) -> usize {
        self.strategies.len()
    }

    pub fn strategy_name(&self, idx: usize) -> Option<String> {
        self.strategies.get(idx).map(|s| s.name.clone())
    }

    pub fn strategy_domain(&self, idx: usize) -> Option<String> {
        self.strategies.get(idx).map(|s| s.domain.clone())
    }

    pub fn tool_count(&self) -> usize {
        BacktestTool::all().len()
    }

    pub fn tool_name(&self, idx: usize) -> Option<String> {
        BacktestTool::all().get(idx).map(|t| t.name().to_string())
    }

    pub fn tool_about(&self, idx: usize) -> Option<String> {
        BacktestTool::all()
            .get(idx)
            .map(|t| t.tooltip().to_string())
    }

    pub fn data_source_count(&self) -> usize {
        self.data_sources.len()
    }

    pub fn data_source_at(&self, idx: usize) -> Option<(String, String)> {
        self.data_sources
            .get(idx)
            .map(|d| (d.id.clone(), d.name.clone()))
    }

    pub fn get_tool_params(&self, tool_idx: usize) -> Vec<ToolParam> {
        self.tool_params
            .read()
            .ok()
            .and_then(|p| p.get(tool_idx).cloned())
            .unwrap_or_default()
    }

    pub fn tool_param_count(&self, tool_idx: usize) -> usize {
        self.get_tool_params(tool_idx).len()
    }

    pub fn adjust_param(&self, tool_idx: usize, param_idx: usize, delta: f64) {
        if let Ok(mut guard) = self.tool_params.write() {
            if let Some(params) = guard.get_mut(tool_idx) {
                if let Some(p) = params.get_mut(param_idx) {
                    p.value = (p.value + delta * p.step).clamp(p.min, p.max);
                }
            }
        }
    }

    pub fn set_param_value(&self, tool_idx: usize, param_idx: usize, val: f64) {
        if let Ok(mut guard) = self.tool_params.write() {
            if let Some(params) = guard.get_mut(tool_idx) {
                if let Some(p) = params.get_mut(param_idx) {
                    p.value = val.clamp(p.min, p.max);
                }
            }
        }
    }

    pub fn get_equity_curve(&self) -> Vec<f64> {
        self.tool_result
            .read()
            .ok()
            .and_then(|r| r.as_ref().map(|tr| tr.equity_curve.clone()))
            .unwrap_or_default()
    }

    pub fn get_perm_curves(&self) -> Option<Vec<Vec<f64>>> {
        self.tool_result.read().ok().and_then(|r| {
            r.as_ref().and_then(|tr| {
                if tr.extra_curves.is_empty() {
                    None
                } else {
                    Some(tr.extra_curves.clone())
                }
            })
        })
    }

    pub fn get_metrics(&self) -> Vec<f64> {
        self.current_metrics
            .read()
            .ok()
            .and_then(|m| m.as_ref().map(|pm| pm.to_vec()))
            .unwrap_or_default()
    }

    pub fn tool_results(&self, _tool_idx: usize) -> Option<(String, Vec<String>)> {
        self.tool_result.read().ok().and_then(|r| {
            r.as_ref().map(|tr| {
                let summary = tr
                    .summary
                    .iter()
                    .map(|(k, v)| format!("{}: {}", k, v))
                    .collect::<Vec<_>>()
                    .join("  ");
                let details = tr
                    .summary
                    .iter()
                    .map(|(k, v)| format!("{}: {}", k, v))
                    .collect();
                (summary, details)
            })
        })
    }

    pub fn is_running_now(&self) -> bool {
        self.is_running.load(Ordering::SeqCst)
    }
}

impl Default for BacktesterState {
    fn default() -> Self {
        let all_params: Vec<Vec<ToolParam>> = BacktestTool::all()
            .iter()
            .map(|t| t.default_params())
            .collect();
        Self {
            strategies: data::load_strategies_with_registry(),
            data_sources: data::default_data_sources(),
            tool_params: RwLock::new(all_params),
            trades: RwLock::new(Vec::new()),
            current_metrics: RwLock::new(None),
            tool_result: RwLock::new(None),
            results: RwLock::new(Vec::new()),
            calib: RwLock::new(CalibResult {
                brier_score: None,
                n_resolved: 0,
                n_pending: 0,
                accuracy: None,
            }),
            is_running: AtomicBool::new(false),
        }
    }
}

/// Load trades from a data provider by converting price series to synthetic trades.
fn load_from_provider(source_id: &str, _strategy_id: &str) -> Vec<Trade> {
    use crate::data_providers::{self, HistoryQuery};
    use crate::shared::strategy::Side;

    let providers = data_providers::all_providers();
    let provider = match providers.iter().find(|p| p.id() == source_id) {
        Some(p) => p,
        None => return Vec::new(),
    };

    // Default query: last 90 days for a generic symbol
    let symbol = match source_id {
        "binance" => "BTCUSDT",
        "polymarket" => "default",
        "espn" => "epl",
        "open_meteo" => "nyc",
        _ => "trades",
    };

    let query = HistoryQuery::last_days(symbol, 90);
    let ts = match provider.fetch_history(&query) {
        Ok(t) => t,
        Err(_) => return Vec::new(),
    };

    // Cache the result
    if let Ok(cache) = data_providers::cache::DataCache::open_default() {
        let _ = cache.store(&ts);
    }

    // Convert price series to trades (price deltas → PnL)
    let mut prices = ts.column("close");
    if prices.is_empty() {
        prices = ts.column("price");
    }
    if prices.len() < 2 {
        return Vec::new();
    }

    prices
        .windows(2)
        .enumerate()
        .map(|(i, w)| {
            let pnl = w[1] - w[0];
            Trade {
                strategy_id: source_id.into(),
                side: if pnl >= 0.0 { Side::Yes } else { Side::No },
                entry_price: w[0],
                exit_price: w[1],
                size: 100.0,
                pnl: pnl * 100.0,
                timestamp: ts
                    .points
                    .get(i + 1)
                    .map(|p| p.timestamp)
                    .unwrap_or(i as i64),
            }
        })
        .collect()
}

/// Legacy tool dispatch — produces BacktestResult with hardcoded params.
fn run_legacy_tool(tool: BacktestTool) -> BacktestResult {
    let now = chrono::Utc::now();
    // Only the simulation tools produce legacy results; others get placeholder
    let (prob, se) = match tool {
        BacktestTool::McBasic => {
            let p = monte_carlo::McParams {
                s0: 1.0,
                k: 0.7,
                mu: 0.0,
                sigma: 0.35,
                t_years: 1.0,
                n_paths: 50_000,
            };
            let r = monte_carlo::simulate_basic(&p);
            (r.probability, r.std_error)
        }
        BacktestTool::McIS => {
            let p = importance_sampling::IsParams {
                s0: 1.0,
                k_crash_pct: 0.3,
                sigma: 0.4,
                t_years: 1.0,
                n_paths: 50_000,
            };
            let r = importance_sampling::simulate_is(&p);
            (r.p_is, r.se_is)
        }
        BacktestTool::AgentBased => {
            let p = abm::AbmParams {
                true_prob: 0.65,
                n_informed: 5,
                n_noise: 20,
                n_mm: 2,
                n_steps: 2000,
            };
            let r = abm::run(&p);
            (r.final_price, r.convergence_error)
        }
        _ => (0.5, 0.01), // Placeholder for non-simulation tools
    };
    BacktestResult {
        tool,
        probability: prob.clamp(0.0, 1.0),
        std_error: se,
        ci_lower: (prob - 1.96 * se).max(0.0),
        ci_upper: (prob + 1.96 * se).min(1.0),
        extra: vec![],
        log: vec![],
        computed_at: now,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_tools_count() {
        assert_eq!(BacktestTool::all().len(), 19);
    }

    #[test]
    fn all_tools_have_nonempty_tooltips() {
        for tool in BacktestTool::all() {
            assert!(
                !tool.tooltip().is_empty(),
                "{} has empty tooltip",
                tool.name()
            );
        }
    }

    #[test]
    fn run_all_completes() {
        let state = BacktesterState::new();
        state.run_all("data/nonexistent.jsonl");
        let results = state.results.read().unwrap();
        assert_eq!(results.len(), 19);
    }

    #[test]
    fn all_legacy_results_valid_probabilities() {
        let state = BacktesterState::new();
        state.run_all("data/nonexistent.jsonl");
        let results = state.results.read().unwrap();
        for r in results.iter() {
            assert!(
                r.probability >= 0.0 && r.probability <= 1.0,
                "{}: probability={}",
                r.tool.name(),
                r.probability
            );
        }
    }

    #[test]
    fn load_trades_synthetic_fallback() {
        let state = BacktesterState::new();
        state.load_trades(0, 1); // "all" + "synthetic"
        let trades = state.trades.read().unwrap();
        assert_eq!(trades.len(), 200);
        let metrics = state.current_metrics.read().unwrap();
        assert!(metrics.is_some());
    }

    #[test]
    fn run_tool_produces_result() {
        let state = BacktesterState::new();
        state.load_trades(0, 1);
        state.run_tool(3); // Historical Replay
        let result = state.tool_result.read().unwrap();
        assert!(result.is_some());
        assert!(!result.as_ref().unwrap().summary.is_empty());
    }

    #[test]
    fn default_params_for_all_tools() {
        for tool in BacktestTool::all() {
            // Should not panic
            let _ = tool.default_params();
        }
    }
}
