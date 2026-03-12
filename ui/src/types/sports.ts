import type { LogEntry } from './shared'

export interface SportsSignal {
  market_id: string
  home: string
  away: string
  date: string
  side: string
  edge_pct: number
  kelly_size: number
  strategy_id: string
  status: string
}

export interface SportsStrategy {
  id: string
  name: string
  description: string
  enabled: boolean
  auto_execute: boolean
}

export interface Fixture {
  home: string
  away: string
  date: string
  home_xg: number
  away_xg: number
  polymarket_id: string | null
}

export interface LiveMatch {
  home_team: string
  away_team: string
  home_goals: number
  away_goals: number
  minute: number
}

export interface SportsState {
  strategies: SportsStrategy[]
  signals: SportsSignal[]
  fixtures: Fixture[]
  live_matches: LiveMatch[]
  logs: LogEntry[]
}
