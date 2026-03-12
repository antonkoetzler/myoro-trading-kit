import { create } from 'zustand'
import { invoke } from '@tauri-apps/api/core'
import type { MmState } from '@/types/mm'

interface MmStore {
  state: MmState | null
  loading: boolean
  refresh: () => Promise<void>
  startMm: () => Promise<void>
  stopMm: () => Promise<void>
  saveConfig: (params: {
    mmHalfSpread: number
    mmMaxInventoryUsd: number
    mmMaxMarkets: number
    mmMinVolumeUsd: number
  }) => Promise<void>
}

export const useMmStore = create<MmStore>((set) => ({
  state: null,
  loading: false,
  refresh: async () => {
    set({ loading: true })
    try {
      const data = await invoke<MmState>('get_mm_state')
      set({ state: data })
    } finally {
      set({ loading: false })
    }
  },
  startMm: async () => {
    await invoke('start_mm')
  },
  stopMm: async () => {
    await invoke('stop_mm')
  },
  saveConfig: async ({ mmHalfSpread, mmMaxInventoryUsd, mmMaxMarkets, mmMinVolumeUsd }) => {
    await invoke('save_mm_config', {
      mmHalfSpread,
      mmMaxInventoryUsd,
      mmMaxMarkets,
      mmMinVolumeUsd,
    })
  },
}))
