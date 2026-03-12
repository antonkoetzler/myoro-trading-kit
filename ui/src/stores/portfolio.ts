import { create } from 'zustand'
import { invoke } from '@tauri-apps/api/core'
import type { PortfolioState } from '@/types/portfolio'

interface PortfolioStore {
  state: PortfolioState | null
  loading: boolean
  refresh: () => Promise<void>
}

export const usePortfolioStore = create<PortfolioStore>((set) => ({
  state: null,
  loading: false,
  refresh: async () => {
    set({ loading: true })
    try {
      const data = await invoke<PortfolioState>('get_portfolio_state')
      set({ state: data })
    } finally {
      set({ loading: false })
    }
  },
}))
