//! Shared application state threaded through Tauri's managed state.

use myoro_trading_kit::backtester::BacktesterState;
use myoro_trading_kit::config::Config;
use myoro_trading_kit::copy_trading::{Monitor, TraderList};
use myoro_trading_kit::live::LiveState;
use std::sync::{atomic::AtomicBool, Arc, RwLock};

pub struct AppState {
    pub config: Arc<RwLock<Config>>,
    pub live: Arc<LiveState>,
    pub backtester: Arc<BacktesterState>,
    pub copy_monitor: Arc<Monitor>,
    pub mm_running: Arc<AtomicBool>,
}

impl AppState {
    pub fn new(config: Config) -> Self {
        let config_arc = Arc::new(RwLock::new(config));
        let live = Arc::new(LiveState::default());
        let backtester = BacktesterState::new();
        let copy_running = Arc::new(AtomicBool::new(false));
        let mm_running = Arc::new(AtomicBool::new(false));
        let trader_list = Arc::new(TraderList::new(Arc::clone(&config_arc)));
        let copy_monitor = Arc::new(Monitor::new(
            trader_list,
            Some(Arc::clone(&live)),
            Arc::clone(&copy_running),
        ));
        Self {
            config: config_arc,
            live,
            backtester,
            copy_monitor,
            mm_running,
        }
    }
}
