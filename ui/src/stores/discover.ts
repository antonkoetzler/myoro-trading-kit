import { create } from 'zustand'
import { invoke } from '@tauri-apps/api/core'
import type { LeaderboardEntry, TraderProfile } from '@/types/discover'

interface DiscoverStore {
  leaderboard: LeaderboardEntry[]
  selectedProfile: TraderProfile | null
  loading: boolean
  refresh: (category?: string, period?: string) => Promise<void>
  loadProfile: (address: string) => Promise<void>
  addToCopy: (address: string) => Promise<boolean>
}

export const useDiscoverStore = create<DiscoverStore>((set) => ({
  leaderboard: [],
  selectedProfile: null,
  loading: false,
  refresh: async (category = 'OVERALL', period = 'WEEK') => {
    set({ loading: true })
    try {
      const data = await invoke<LeaderboardEntry[]>('fetch_discover_leaderboard', { category, period })
      set({ leaderboard: data })
    } finally {
      set({ loading: false })
    }
  },
  loadProfile: async (address) => {
    const profile = await invoke<TraderProfile | null>('get_trader_profile', { address })
    set({ selectedProfile: profile })
  },
  addToCopy: async (address) => {
    return invoke<boolean>('add_trader_to_copy', { address })
  },
}))
