import { create } from 'zustand'
import type { GlobalStats, Config } from '@/types/shared'

interface GlobalStore {
  activeTab: number
  theme: 'dark' | 'light' | 'high-contrast'
  paletteOpen: boolean
  shortcutsOpen: boolean
  settingsOpen: boolean
  themeOpen: boolean
  globalStats: GlobalStats | null
  config: Config | null

  setActiveTab: (tab: number) => void
  setTheme: (theme: 'dark' | 'light' | 'high-contrast') => void
  openPalette: () => void
  openShortcuts: () => void
  openSettings: () => void
  openTheme: () => void
  closeAllDialogs: () => void
  setGlobalStats: (stats: GlobalStats) => void
  setConfig: (cfg: Config) => void
}

export const useGlobalStore = create<GlobalStore>((set) => ({
  activeTab: 0,
  theme: (localStorage.getItem('theme') as GlobalStore['theme']) ?? 'dark',
  paletteOpen: false,
  shortcutsOpen: false,
  settingsOpen: false,
  themeOpen: false,
  globalStats: null,
  config: null,

  setActiveTab: (tab) => set({ activeTab: tab }),
  setTheme: (theme) => {
    localStorage.setItem('theme', theme)
    document.documentElement.setAttribute('data-theme', theme)
    set({ theme })
  },
  openPalette: () => set({ paletteOpen: true, shortcutsOpen: false, settingsOpen: false }),
  openShortcuts: () => set({ shortcutsOpen: true, paletteOpen: false }),
  openSettings: () => set({ settingsOpen: true, paletteOpen: false }),
  openTheme: () => set({ themeOpen: true }),
  closeAllDialogs: () =>
    set({ paletteOpen: false, shortcutsOpen: false, settingsOpen: false, themeOpen: false }),
  setGlobalStats: (stats) => set({ globalStats: stats }),
  setConfig: (cfg) => set({ config: cfg }),
}))
