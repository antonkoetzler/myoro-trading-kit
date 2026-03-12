export interface Position {
  market_id: string
  outcome: string
  size: number
  avg_price: number
  current_value: number
}

export interface TradeRow {
  timestamp: string
  domain: string
  market_id: string
  side: string
  size: number
  price: number
  status: string
}

export interface DomainPnl {
  domain: string
  today_pnl: number
  alltime_pnl: number
}

export interface PortfolioState {
  open_positions: Position[]
  trade_history: TradeRow[]
  domain_pnl: DomainPnl[]
  total_pnl: number
}
