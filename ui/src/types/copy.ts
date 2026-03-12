import type { LogEntry } from './shared'

export interface CopyTradeRow {
  user: string
  side: string
  size: number
  price: number
  title: string
  outcome: string
  ts: number
  tx: string
}

export interface CopyState {
  traders: string[]
  is_running: boolean
  recent_trades: CopyTradeRow[]
  logs: LogEntry[]
}
