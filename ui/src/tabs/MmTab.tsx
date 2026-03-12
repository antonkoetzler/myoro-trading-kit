import { useMmStore } from '@/stores/mm'
import { usePoll } from '@/hooks/usePoll'
import { DataTable } from '@/components/shared/DataTable'
import { MetricCard } from '@/components/shared/MetricCard'
import { useState } from 'react'
import { useGlobalStore } from '@/stores/global'

export function MmTab() {
  const { state, refresh, startMm, stopMm, saveConfig } = useMmStore()
  const { config } = useGlobalStore()
  const [halfSpread, setHalfSpread] = useState(config?.mm_half_spread ?? 0.02)
  const [maxInv, setMaxInv] = useState(config?.mm_max_inventory_usd ?? 200)
  const [maxMarkets, setMaxMarkets] = useState(config?.mm_max_markets ?? 5)
  const [minVol, setMinVol] = useState(config?.mm_min_volume_usd ?? 1000)

  usePoll(refresh, 4000)

  if (!state) return <div className="p-4 text-muted-foreground">Loading…</div>

  return (
    <div className="flex h-full overflow-hidden">
      {/* Left: config + controls */}
      <div className="w-72 border-r border-border p-4 flex flex-col gap-4 overflow-auto">
        <div className="flex items-center justify-between">
          <div className="text-xs font-bold text-muted-foreground uppercase">Market Making</div>
          <button
            onClick={state.running ? stopMm : startMm}
            className={`text-xs px-3 py-1 rounded ${state.running ? 'bg-destructive text-destructive-foreground' : 'bg-success/20 text-success'}`}
          >
            {state.running ? 'Stop' : 'Start'}
          </button>
        </div>

        <MetricCard label="Total Realized P&L" value={state.total_realized_pnl} />
        <MetricCard label="Fill Count" value={state.fill_count} positive={true} />

        <div className="text-xs font-bold text-muted-foreground uppercase">Config</div>
        {[
          { label: 'Half Spread', value: halfSpread, set: setHalfSpread, min: 0.001, max: 0.5, step: 0.001 },
          { label: 'Max Inventory USD', value: maxInv, set: setMaxInv, min: 0, max: 10000, step: 10 },
          { label: 'Max Markets', value: maxMarkets, set: setMaxMarkets, min: 1, max: 50, step: 1 },
          { label: 'Min Volume USD', value: minVol, set: setMinVol, min: 0, max: 50000, step: 100 },
        ].map(({ label, value, set, min, max, step }) => (
          <div key={label}>
            <label className="text-xs text-muted-foreground">{label}</label>
            <input
              type="number" min={min} max={max} step={step} value={value}
              onChange={(e) => set(parseFloat(e.target.value))}
              className="w-full text-xs bg-input border border-border rounded px-2 py-1 mt-1"
            />
          </div>
        ))}
        <button
          onClick={() => saveConfig({ mmHalfSpread: halfSpread, mmMaxInventoryUsd: maxInv, mmMaxMarkets: maxMarkets, mmMinVolumeUsd: minVol })}
          className="text-xs bg-primary text-primary-foreground px-3 py-2 rounded"
        >
          Save Config
        </button>
      </div>

      {/* Center: active quotes */}
      <div className="flex-1 flex flex-col overflow-hidden border-r border-border">
        <div className="text-xs font-bold text-muted-foreground uppercase p-3 border-b border-border">
          Active Quotes ({state.active_quotes.length})
        </div>
        <DataTable
          columns={[
            { key: 'market_id', header: 'Market' },
            { key: 'side', header: 'Side' },
            { key: 'price', header: 'Price', render: (r) => r.price.toFixed(3) },
            { key: 'size', header: 'Size', render: (r) => r.size.toFixed(2) },
          ]}
          data={state.active_quotes}
          emptyMessage="No active quotes"
        />
      </div>

      {/* Right: inventory */}
      <div className="w-72 overflow-auto">
        <div className="text-xs font-bold text-muted-foreground uppercase p-3 border-b border-border">
          Inventory
        </div>
        <DataTable
          columns={[
            { key: 'market_id', header: 'Market' },
            { key: 'net_yes', header: 'Net YES', render: (r) => r.net_yes.toFixed(2) },
            { key: 'realized_pnl', header: 'P&L', render: (r) => r.realized_pnl.toFixed(2) },
          ]}
          data={state.inventory}
          emptyMessage="No inventory"
        />
      </div>
    </div>
  )
}
