import { create } from 'zustand'
import { invoke } from '@tauri-apps/api/core'
import type { CryptoSignal } from '@/types/crypto'
import type { SportsSignal } from '@/types/sports'
import type { WeatherSignal } from '@/types/weather'

interface AllSignals {
  crypto: CryptoSignal[]
  sports: SportsSignal[]
  weather: WeatherSignal[]
}

interface SignalsStore {
  signals: AllSignals
  loading: boolean
  refresh: () => Promise<void>
}

export const useSignalsStore = create<SignalsStore>((set) => ({
  signals: { crypto: [], sports: [], weather: [] },
  loading: false,
  refresh: async () => {
    set({ loading: true })
    try {
      const data = await invoke<AllSignals>('get_all_signals')
      set({ signals: data })
    } finally {
      set({ loading: false })
    }
  },
}))
