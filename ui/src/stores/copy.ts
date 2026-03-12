import { create } from 'zustand'
import { invoke } from '@tauri-apps/api/core'
import type { CopyState } from '@/types/copy'

interface CopyStore {
  state: CopyState | null
  loading: boolean
  refresh: () => Promise<void>
  addTrader: (address: string) => Promise<boolean>
  removeTrader: (idx: number) => Promise<void>
  startCopy: () => Promise<void>
  stopCopy: () => Promise<void>
}

export const useCopyStore = create<CopyStore>((set) => ({
  state: null,
  loading: false,
  refresh: async () => {
    set({ loading: true })
    try {
      const data = await invoke<CopyState>('get_copy_state')
      set({ state: data })
    } finally {
      set({ loading: false })
    }
  },
  addTrader: async (address) => {
    const ok = await invoke<boolean>('add_copy_trader', { address })
    return ok
  },
  removeTrader: async (idx) => {
    await invoke('remove_copy_trader', { idx })
  },
  startCopy: async () => {
    await invoke('start_copy_trading')
  },
  stopCopy: async () => {
    await invoke('stop_copy_trading')
  },
}))
