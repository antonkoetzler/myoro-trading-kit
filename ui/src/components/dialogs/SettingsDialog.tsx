import { useState } from 'react'
import { useGlobalStore } from '@/stores/global'
import { invoke } from '@tauri-apps/api/core'

type Section = 'general' | 'risk' | 'copy' | 'mm'

export function SettingsDialog() {
  const { settingsOpen, closeAllDialogs, config } = useGlobalStore()
  const [section, setSection] = useState<Section>('general')
  const [saved, setSaved] = useState(false)

  if (!settingsOpen || !config) return null

  const save = async () => {
    await invoke('save_config_settings', {
      paperBankroll: config.paper_bankroll,
      pnlCurrency: config.pnl_currency,
      maxDailyLossUsd: config.max_daily_loss_usd,
      maxPositionUsd: config.max_position_usd,
      maxOpenPositions: config.max_open_positions,
      mmEnabled: config.mm_enabled,
      mmHalfSpread: config.mm_half_spread,
      mmMaxInventoryUsd: config.mm_max_inventory_usd,
      mmMaxMarkets: config.mm_max_markets,
      mmMinVolumeUsd: config.mm_min_volume_usd,
      binanceLagAssets: config.binance_lag_assets,
    })
    setSaved(true)
    setTimeout(() => setSaved(false), 2000)
  }

  const sections: Section[] = ['general', 'risk', 'copy', 'mm']

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-background/80 backdrop-blur-sm">
      <div className="bg-card border border-border rounded-lg shadow-xl w-full max-w-xl">
        <div className="flex items-center justify-between p-4 border-b border-border">
          <div className="font-bold text-sm">Settings</div>
          <button onClick={closeAllDialogs} className="text-muted-foreground hover:text-foreground">×</button>
        </div>
        <div className="flex">
          <nav className="w-32 border-r border-border p-2 flex flex-col gap-1">
            {sections.map((s) => (
              <button
                key={s}
                onClick={() => setSection(s)}
                className={`text-xs text-left px-2 py-1.5 rounded capitalize ${section === s ? 'bg-primary text-primary-foreground' : 'text-muted-foreground hover:text-foreground'}`}
              >
                {s}
              </button>
            ))}
          </nav>
          <div className="flex-1 p-4 text-xs text-muted-foreground">
            [{section} settings — connect to config fields]
          </div>
        </div>
        <div className="flex items-center justify-between p-4 border-t border-border">
          {saved && <span className="text-xs text-success">Saved!</span>}
          <div className="flex gap-2 ml-auto">
            <button onClick={closeAllDialogs} className="text-xs bg-secondary text-secondary-foreground px-3 py-1.5 rounded">Cancel</button>
            <button onClick={save} className="text-xs bg-primary text-primary-foreground px-3 py-1.5 rounded">Save</button>
          </div>
        </div>
      </div>
    </div>
  )
}
