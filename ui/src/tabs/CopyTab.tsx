import { useState } from 'react'
import { useCopyStore } from '@/stores/copy'
import { usePoll } from '@/hooks/usePoll'
import { DataTable } from '@/components/shared/DataTable'

export function CopyTab() {
  const { state, refresh, addTrader, removeTrader, startCopy, stopCopy } = useCopyStore()
  const [newAddress, setNewAddress] = useState('')
  usePoll(refresh, 4000)

  if (!state) return <div className="p-4 text-muted-foreground">Loading…</div>

  const handleAdd = async () => {
    if (newAddress.trim()) {
      await addTrader(newAddress.trim())
      setNewAddress('')
      await refresh()
    }
  }

  return (
    <div className="flex h-full overflow-hidden">
      {/* Left: trader list + controls */}
      <div className="w-72 border-r border-border flex flex-col p-3 gap-3">
        <div className="flex items-center justify-between">
          <div className="text-xs font-bold text-muted-foreground uppercase">Copy Traders</div>
          <button
            onClick={state.is_running ? stopCopy : startCopy}
            className={`text-xs px-2 py-1 rounded ${state.is_running ? 'bg-destructive text-destructive-foreground' : 'bg-success/20 text-success'}`}
          >
            {state.is_running ? 'Stop' : 'Start'}
          </button>
        </div>

        <div className="flex gap-2">
          <input
            value={newAddress}
            onChange={(e) => setNewAddress(e.target.value)}
            onKeyDown={(e) => e.key === 'Enter' && handleAdd()}
            placeholder="0x address…"
            className="flex-1 text-xs bg-input border border-border rounded px-2 py-1 text-foreground"
          />
          <button onClick={handleAdd} className="text-xs bg-primary text-primary-foreground px-2 py-1 rounded">
            Add
          </button>
        </div>

        {state.traders.map((addr, i) => (
          <div key={addr} className="flex items-center justify-between text-xs">
            <span className="font-mono truncate text-foreground">{addr.slice(0, 10)}…{addr.slice(-6)}</span>
            <button
              onClick={async () => { await removeTrader(i); await refresh() }}
              className="text-destructive hover:text-destructive/80 ml-2"
            >
              ×
            </button>
          </div>
        ))}
      </div>

      {/* Right: trade feed */}
      <div className="flex-1 overflow-auto">
        <DataTable
          columns={[
            { key: 'user', header: 'Trader', render: (r) => `${r.user.slice(0, 8)}…` },
            { key: 'title', header: 'Market' },
            { key: 'side', header: 'Side' },
            { key: 'size', header: 'Size', render: (r) => r.size.toFixed(2) },
            { key: 'price', header: 'Price', render: (r) => r.price.toFixed(3) },
          ]}
          data={state.recent_trades}
          emptyMessage="No trades yet — start copy trading to see live feed"
        />
      </div>
    </div>
  )
}
