import type { LogEntry } from './shared'

export interface CryptoSignal {
  market_id: string
  label: string
  side: string
  edge_pct: number
  kelly_size: number
  strategy_id: string
  status: string
  created_at: string
}

export interface CryptoStrategy {
  id: string
  name: string
  description: string
  enabled: boolean
  auto_execute: boolean
}

export interface GammaMarket {
  id: string
  title: string
  best_bid: number
  best_ask: number
  volume: number
}

export interface CryptoState {
  btc_usdt: string
  events: string[]
  strategies: CryptoStrategy[]
  signals: CryptoSignal[]
  markets: GammaMarket[]
  binance_lag_confidence: number[]
  logical_arb_confidence: number[]
  logs: LogEntry[]
}
