import { useState } from 'react'
import { useDiscoverStore } from '@/stores/discover'
import { usePoll } from '@/hooks/usePoll'
import { DataTable } from '@/components/shared/DataTable'

export function DiscoverTab() {
  const { leaderboard, selectedProfile, loading, error, refresh, loadProfile, addToCopy } = useDiscoverStore()
  const [category, setCategory] = useState('OVERALL')
  const [period, setPeriod] = useState('WEEK')
  const [addedMsg, setAddedMsg] = useState<string | null>(null)

  usePoll(() => refresh(category, period), 30000)

  const handleAdd = async (addr: string) => {
    const ok = await addToCopy(addr)
    setAddedMsg(ok ? 'Added to copy list' : 'Already in list or invalid address')
    setTimeout(() => setAddedMsg(null), 3000)
  }

  return (
    <div className="flex h-full overflow-hidden">
      <div className="flex-1 flex flex-col overflow-hidden">
        <div className="flex items-center gap-3 p-3 border-b border-border">
          <select
            value={category}
            onChange={(e) => setCategory(e.target.value)}
            className="text-xs bg-input border border-border rounded px-2 py-1 outline-none focus:ring-1 focus:ring-ring"
          >
            {['OVERALL', 'CRYPTO', 'SPORTS', 'POLITICS', 'WEATHER'].map((c) => <option key={c}>{c}</option>)}
          </select>
          <select
            value={period}
            onChange={(e) => setPeriod(e.target.value)}
            className="text-xs bg-input border border-border rounded px-2 py-1 outline-none focus:ring-1 focus:ring-ring"
          >
            {['DAY', 'WEEK', 'MONTH', 'ALL'].map((p) => <option key={p}>{p}</option>)}
          </select>
          <button
            onClick={() => refresh(category, period)}
            disabled={loading}
            className="text-xs bg-secondary text-secondary-foreground px-3 py-1 rounded hover:bg-secondary/80 disabled:opacity-50 transition-colors"
          >
            {loading ? 'Loading…' : 'Refresh'}
          </button>
          {addedMsg && (
            <span className={`text-xs ${addedMsg.startsWith('Added') ? 'text-success' : 'text-destructive'}`}>
              {addedMsg}
            </span>
          )}
        </div>

        {error ? (
          <div className="flex flex-col items-center justify-center flex-1 gap-2 p-6">
            <div className="flex items-center gap-2 text-destructive">
              <span>⚠</span>
              <span className="text-sm font-medium">Failed to load leaderboard</span>
            </div>
            <p className="text-xs text-muted-foreground text-center max-w-sm">{error}</p>
            <button
              onClick={() => refresh(category, period)}
              className="mt-2 text-xs bg-secondary text-secondary-foreground px-4 py-1.5 rounded hover:bg-secondary/80 transition-colors"
            >
              Retry
            </button>
          </div>
        ) : (
          <DataTable
            className="flex-1"
            columns={[
              { key: 'rank', header: '#', className: 'w-12' },
              { key: 'user_name', header: 'Name' },
              { key: 'pnl', header: 'P&L', render: (r) => `$${r.pnl.toFixed(0)}` },
              { key: 'vol', header: 'Volume', render: (r) => `$${r.vol.toFixed(0)}` },
              { key: 'proxy_wallet', header: 'Address', render: (r) => `${r.proxy_wallet.slice(0, 8)}…` },
            ]}
            data={leaderboard}
            onSelect={(i) => leaderboard[i] && loadProfile(leaderboard[i].proxy_wallet)}
            emptyMessage="Click Refresh to load leaderboard"
          />
        )}
      </div>

      {selectedProfile && (
        <div className="w-72 border-l border-border p-4 flex flex-col gap-3">
          <div className="text-xs font-bold text-muted-foreground uppercase">Trader Profile</div>
          <div className="font-mono text-xs break-all text-muted-foreground">{selectedProfile.address}</div>
          <div className="flex flex-col gap-1.5">
            <div className="flex justify-between text-sm">
              <span className="text-muted-foreground">Trades</span>
              <span>{selectedProfile.trade_count}</span>
            </div>
            <div className="flex justify-between text-sm">
              <span className="text-muted-foreground">Top Category</span>
              <span>{selectedProfile.top_category}</span>
            </div>
            {selectedProfile.win_rate !== null && (
              <div className="flex justify-between text-sm">
                <span className="text-muted-foreground">Win Rate</span>
                <span className="text-success">{((selectedProfile.win_rate ?? 0) * 100).toFixed(1)}%</span>
              </div>
            )}
          </div>
          <button
            onClick={() => handleAdd(selectedProfile.address)}
            className="text-sm bg-primary text-primary-foreground px-3 py-2 rounded mt-auto hover:bg-primary/90 transition-colors"
          >
            Add to Copy Trading
          </button>
        </div>
      )}
    </div>
  )
}
