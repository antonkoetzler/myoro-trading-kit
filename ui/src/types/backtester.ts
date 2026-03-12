export interface ToolParam {
  name: string
  value: number
  min: number
  max: number
  step: number
}

export interface BacktestTool {
  idx: number
  name: string
  tooltip: string
  params: ToolParam[]
}

export interface StrategyEntry {
  id: string
  name: string
  domain: string
}

export interface DataSource {
  id: string
  name: string
  domain: string
}

export interface BacktestTradeRow {
  strategy_id: string
  side: string
  entry_price: number
  exit_price: number
  size: number
  pnl: number
  timestamp: number
}

export interface BacktesterState {
  strategies: StrategyEntry[]
  data_sources: DataSource[]
  tools: BacktestTool[]
  is_running: boolean
}

export interface BacktesterResults {
  equity_curve: number[]
  drawdown_curve: number[]
  pnl_buckets: [number, number][]
  mc_paths: number[][] | null
  metrics: Record<string, number>
  trade_list: BacktestTradeRow[]
  is_running: boolean
  last_error: string | null
  tool_extra: [string, string][]
}
