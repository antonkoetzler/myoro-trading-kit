import type { LogEntry } from './shared'

export interface ActiveQuote {
  market_id: string
  side: string
  price: number
  size: number
  order_id: string
}

export interface InventoryEntry {
  market_id: string
  net_yes: number
  realized_pnl: number
  volume: number
}

export interface MmState {
  active_quotes: ActiveQuote[]
  inventory: InventoryEntry[]
  total_realized_pnl: number
  fill_count: number
  running: boolean
  logs: LogEntry[]
}
