export interface LeaderboardEntry {
  rank: string
  proxy_wallet: string
  user_name: string
  vol: number
  pnl: number
}

export interface TraderProfile {
  address: string
  trade_count: number
  top_category: string
  win_rate: number | null
}

export interface DiscoverState {
  leaderboard: LeaderboardEntry[]
  selected_profile: TraderProfile | null
  logs: { level: string; message: string }[]
}
