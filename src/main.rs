mod app_state;
mod commands;

use app_state::AppState;
use std::sync::{atomic::Ordering, Arc};

fn main() {
    let config = myoro_trading_kit::config::load().expect("config load");
    let state = AppState::new(config);

    // Clone Arcs before state is moved into Tauri's manage()
    let live_bg = Arc::clone(&state.live);
    let config_bg = Arc::clone(&state.config);
    let copy_bg = Arc::clone(&state.copy_monitor);
    let config_copy = Arc::clone(&state.config);
    let live_mm = Arc::clone(&state.live);
    let config_mm = Arc::clone(&state.config);
    let mm_running = Arc::clone(&state.mm_running);

    tauri::Builder::default()
        .manage(state)
        .setup(move |_app| {
            // Thread 1: Live data poller (8s)
            std::thread::spawn(move || loop {
                let cfg = config_bg.read().map(|c| c.clone()).unwrap_or_default();
                live_bg.fetch_all_with_config(&cfg);
                std::thread::sleep(std::time::Duration::from_secs(8));
            });

            // Thread 2: Copy trading poller
            std::thread::spawn(move || loop {
                let poll_ms = config_copy.read().map(|c| c.copy_poll_ms).unwrap_or(250);
                copy_bg.poll_once();
                std::thread::sleep(std::time::Duration::from_millis(poll_ms));
            });

            // Thread 3: MM cycle (30s, only runs when mm_running=true)
            std::thread::spawn(move || loop {
                if mm_running.load(Ordering::SeqCst) {
                    let cfg = config_mm.read().map(|c| c.clone()).unwrap_or_default();
                    live_mm.run_mm(&cfg);
                }
                std::thread::sleep(std::time::Duration::from_secs(30));
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::config::get_config,
            commands::config::save_config_settings,
            commands::global::get_global_stats,
            commands::global::get_logs,
            commands::global::reset_circuit_breaker,
            commands::crypto::get_crypto_state,
            commands::crypto::toggle_crypto_strategy,
            commands::crypto::dismiss_crypto_signal,
            commands::sports::get_sports_state,
            commands::sports::toggle_sports_strategy,
            commands::sports::dismiss_sports_signal,
            commands::weather::get_weather_state,
            commands::weather::toggle_weather_strategy,
            commands::portfolio::get_portfolio_state,
            commands::copy::get_copy_state,
            commands::copy::add_copy_trader,
            commands::copy::remove_copy_trader,
            commands::copy::start_copy_trading,
            commands::copy::stop_copy_trading,
            commands::discover::fetch_discover_leaderboard,
            commands::discover::get_trader_profile,
            commands::discover::add_trader_to_copy,
            commands::signals::get_all_signals,
            commands::backtester::get_backtester_state,
            commands::backtester::backtester_load_trades,
            commands::backtester::backtester_run_tool,
            commands::backtester::backtester_set_param,
            commands::backtester::get_backtester_results,
            commands::mm::get_mm_state,
            commands::mm::start_mm,
            commands::mm::stop_mm,
            commands::mm::save_mm_config,
        ])
        .run(tauri::generate_context!())
        .expect("tauri error");
}
