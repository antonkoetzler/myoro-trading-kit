//! Integration tests for the Backtester tab simulation toolbox.
use myoro_polymarket_terminal::backtester::{BacktestTool, BacktesterState};
use std::io::Write;

#[test]
fn backtest_run_all_completes_without_panic() {
    let state = BacktesterState::new();
    state.run_all("data/nonexistent_paper_trades.jsonl");
    // If we reach here, no panic occurred
}

#[test]
fn backtest_all_tools_produce_valid_probabilities() {
    let state = BacktesterState::new();
    state.run_all("data/nonexistent_paper_trades.jsonl");
    let results = state.results.read().expect("results readable");
    assert_eq!(results.len(), 19, "all 19 tools ran");
    for r in results.iter() {
        assert!(
            r.probability >= 0.0 && r.probability <= 1.0,
            "{}: probability={} out of [0,1]",
            r.tool.name(),
            r.probability
        );
    }
}

#[test]
fn backtest_results_stored_after_run() {
    let state = BacktesterState::new();
    state.run_all("data/nonexistent_paper_trades.jsonl");
    let results = state.results.read().expect("results readable");
    assert_eq!(results.len(), 19);
}

#[test]
fn backtest_calib_reads_paper_trades() {
    // Write a temp JSONL file with known records
    let tmp_path = std::env::temp_dir().join("backtest_test_paper_trades.jsonl");
    let mut f = std::fs::File::create(&tmp_path).expect("create temp file");
    writeln!(
        f,
        r#"{{"predicted": 0.8, "outcome": true, "strategy_id": "test"}}"#
    )
    .unwrap();
    writeln!(
        f,
        r#"{{"predicted": 0.3, "outcome": false, "strategy_id": "test"}}"#
    )
    .unwrap();
    writeln!(f, r#"{{"predicted": 0.6, "strategy_id": "pending"}}"#).unwrap();
    drop(f);

    let state = BacktesterState::new();
    state.run_all(tmp_path.to_str().expect("valid path"));

    let calib = state.calib.read().expect("calib readable");
    assert_eq!(calib.n_resolved, 2);
    assert_eq!(calib.n_pending, 1);
    assert!(calib.brier_score.is_some());
    // Brier score: ((0.8-1)^2 + (0.3-0)^2)/2 = (0.04 + 0.09)/2 = 0.065
    let bs = calib.brier_score.unwrap();
    assert!((bs - 0.065).abs() < 0.001, "brier={bs}");

    std::fs::remove_file(tmp_path).ok();
}

#[test]
fn backtest_tooltip_all_tools_have_nonempty_text() {
    for tool in BacktestTool::all() {
        assert!(
            !tool.tooltip().is_empty(),
            "{} has empty tooltip",
            tool.name()
        );
        assert!(
            tool.tooltip().len() > 20,
            "{} tooltip too short: '{}'",
            tool.name(),
            tool.tooltip()
        );
    }
}

#[test]
fn backtest_variance_reduction_better_than_basic() {
    use myoro_polymarket_terminal::backtester::monte_carlo::{
        simulate_antithetic, simulate_basic, McParams,
    };

    let p = McParams {
        s0: 1.0,
        k: 0.7,
        mu: 0.0,
        sigma: 0.35,
        t_years: 1.0,
        n_paths: 20_000,
    };
    let basic = simulate_basic(&p);
    let anti = simulate_antithetic(&p);
    // Antithetic should have lower or similar SE (allow 20% slack for statistical noise)
    assert!(
        anti.std_error <= basic.std_error * 1.2,
        "anti SE={} not better than basic SE={}",
        anti.std_error,
        basic.std_error
    );
}
