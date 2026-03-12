import { useEffect } from 'react'
import { invoke } from '@tauri-apps/api/core'
import { useGlobalStore } from '@/stores/global'
import { useKeyboard } from '@/hooks/useKeyboard'
import { Header } from '@/components/layout/Header'
import { TabBar } from '@/components/layout/TabBar'
import { StatusBar } from '@/components/layout/StatusBar'
import { CommandPalette } from '@/components/dialogs/CommandPalette'
import { ShortcutsDialog } from '@/components/dialogs/ShortcutsDialog'
import { SettingsDialog } from '@/components/dialogs/SettingsDialog'
import { CryptoTab } from '@/tabs/CryptoTab'
import { SportsTab } from '@/tabs/SportsTab'
import { WeatherTab } from '@/tabs/WeatherTab'
import { CopyTab } from '@/tabs/CopyTab'
import { PortfolioTab } from '@/tabs/PortfolioTab'
import { SignalsTab } from '@/tabs/SignalsTab'
import { DiscoverTab } from '@/tabs/DiscoverTab'
import { BacktesterTab } from '@/tabs/backtester'
import { MmTab } from '@/tabs/MmTab'
import type { GlobalStats, Config } from '@/types/shared'

const TABS = [
  <CryptoTab />,
  <SportsTab />,
  <WeatherTab />,
  <CopyTab />,
  <PortfolioTab />,
  <SignalsTab />,
  <DiscoverTab />,
  <BacktesterTab />,
  <MmTab />,
]

export default function App() {
  const { activeTab, theme, setGlobalStats, setConfig } = useGlobalStore()
  useKeyboard()

  // Apply theme on mount
  useEffect(() => {
    document.documentElement.setAttribute('data-theme', theme)
  }, [theme])

  // Poll global stats every 4s
  useEffect(() => {
    const load = async () => {
      try {
        const stats = await invoke<GlobalStats>('get_global_stats')
        setGlobalStats(stats)
      } catch { /* backend not ready yet */ }
      try {
        const cfg = await invoke<Config>('get_config')
        setConfig(cfg)
      } catch { /* backend not ready yet */ }
    }
    load()
    const id = setInterval(load, 4000)
    return () => clearInterval(id)
  }, [setGlobalStats, setConfig])

  return (
    <div className="flex flex-col h-screen overflow-hidden bg-background text-foreground">
      <Header />
      <TabBar />
      <main className="flex-1 overflow-hidden">
        {TABS[activeTab]}
      </main>
      <StatusBar />

      <CommandPalette />
      <ShortcutsDialog />
      <SettingsDialog />
    </div>
  )
}
