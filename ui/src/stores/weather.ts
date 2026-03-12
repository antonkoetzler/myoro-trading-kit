import { create } from 'zustand'
import { invoke } from '@tauri-apps/api/core'
import type { WeatherState } from '@/types/weather'

interface WeatherStore {
  state: WeatherState | null
  loading: boolean
  refresh: () => Promise<void>
  toggleStrategy: (idx: number, enabled: boolean) => Promise<void>
}

export const useWeatherStore = create<WeatherStore>((set) => ({
  state: null,
  loading: false,
  refresh: async () => {
    set({ loading: true })
    try {
      const data = await invoke<WeatherState>('get_weather_state')
      set({ state: data })
    } finally {
      set({ loading: false })
    }
  },
  toggleStrategy: async (idx, enabled) => {
    await invoke('toggle_weather_strategy', { idx, enabled })
  },
}))
