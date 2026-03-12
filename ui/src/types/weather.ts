import type { LogEntry } from './shared'

export interface WeatherSignal {
  market_id: string
  city: string
  label: string
  side: string
  edge_pct: number
  kelly_size: number
  strategy_id: string
  status: string
}

export interface WeatherStrategy {
  id: string
  name: string
  description: string
  enabled: boolean
  auto_execute: boolean
}

export interface CityForecast {
  city: string
  today_max_c: number | null
  today_min_c: number | null
  market_implied: number | null
  market_id: string | null
}

export interface WeatherState {
  forecast: string[]
  strategies: WeatherStrategy[]
  signals: WeatherSignal[]
  city_forecasts: CityForecast[]
  logs: LogEntry[]
}
