import { create } from 'zustand'
import { invoke } from '@tauri-apps/api/core'
import type { CryptoState } from '@/types/crypto'

interface CryptoStore {
  state: CryptoState | null
  loading: boolean
  refresh: () => Promise<void>
  toggleStrategy: (idx: number, enabled: boolean) => Promise<void>
  dismissSignal: (marketId: string) => Promise<void>
}

export const useCryptoStore = create<CryptoStore>((set) => ({
  state: null,
  loading: false,
  refresh: async () => {
    set({ loading: true })
    try {
      const data = await invoke<CryptoState>('get_crypto_state')
      set({ state: data })
    } finally {
      set({ loading: false })
    }
  },
  toggleStrategy: async (idx, enabled) => {
    await invoke('toggle_crypto_strategy', { idx, enabled })
  },
  dismissSignal: async (marketId) => {
    await invoke('dismiss_crypto_signal', { marketId })
  },
}))
