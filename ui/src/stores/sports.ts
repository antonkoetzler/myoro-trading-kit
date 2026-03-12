import { create } from 'zustand'
import { invoke } from '@tauri-apps/api/core'
import type { SportsState } from '@/types/sports'

interface SportsStore {
  state: SportsState | null
  loading: boolean
  refresh: () => Promise<void>
  toggleStrategy: (idx: number, enabled: boolean) => Promise<void>
  dismissSignal: (marketId: string) => Promise<void>
}

export const useSportsStore = create<SportsStore>((set) => ({
  state: null,
  loading: false,
  refresh: async () => {
    set({ loading: true })
    try {
      const data = await invoke<SportsState>('get_sports_state')
      set({ state: data })
    } finally {
      set({ loading: false })
    }
  },
  toggleStrategy: async (idx, enabled) => {
    await invoke('toggle_sports_strategy', { idx, enabled })
  },
  dismissSignal: async (marketId) => {
    await invoke('dismiss_sports_signal', { marketId })
  },
}))
