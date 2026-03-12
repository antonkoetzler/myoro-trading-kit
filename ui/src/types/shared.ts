export interface LogEntry {
  level: 'info' | 'success' | 'warning' | 'error'
  message: string
}

export interface GlobalStats {
  bankroll: number | null
  pnl: number
  open_trades: number
  closed_trades: number
  daily_loss_usd: number
  circuit_breaker_active: boolean
}

export interface Config {
  execution_mode: 'paper' | 'live'
  paper_bankroll: number | null
  pnl_currency: string
  copy_traders: string[]
  copy_poll_ms: number
  copy_bankroll_fraction: number
  copy_max_usd: number
  copy_auto_execute: boolean
  max_daily_loss_usd: number
  max_position_usd: number
  max_open_positions: number
  mm_enabled: boolean
  mm_half_spread: number
  mm_max_inventory_usd: number
  mm_max_markets: number
  mm_min_volume_usd: number
  binance_lag_assets: string[]
}
