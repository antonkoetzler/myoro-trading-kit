import { useState } from 'react'
import { useDiscoverStore } from '@/stores/discover'
import { usePoll } from '@/hooks/usePoll'
import { DataTable } from '@/components/shared/DataTable'

export function DiscoverTab() {
  const { leaderboard, selectedProfile, refresh, loadProfile, addToCopy } = useDiscoverStore()
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
          <select value={category} onChange={(e) => setCategory(e.target.value)} className="text-xs bg-input border border-border rounded px-2 py-1">
            {['OVERALL','CRYPTO','SPORTS','POLITICS','WEATHER'].map((c) => <option key={c}>{c}</option>)}
          </select>
          <select value={period} onChange={(e) => setPeriod(e.target.value)} className="text-xs bg-input border border-border rounded px-2 py-1">
            {['DAY','WEEK','MONTH','ALL'].map((p) => <option key={p}>{p}</option>)}
          </select>
          <button onClick={() => refresh(category, period)} className="text-xs bg-secondary text-secondary-foreground px-3 py-1 rounded">Refresh</button>
          {addedMsg && <span className="text-xs text-success">{addedMsg}</span>}
        </div>
        <DataTable
          className="flex-1"
          columns={[
            { key: 'rank', header: '#', className: 'w-12' },
            { key: 'user_name', header: 'Name' },
            { key: 'pnl', header: 'P&L', render: (r) => `$${r.pnl.toFixed(0)}` },
            { key: 'vol', header: 'Volume', render: (r) => `$${r.vol.toFixed(0)}` },
            { key: 'proxy_wallet', header: 'Address', render: (r) => `${r.proxy_wallet.slice(0,8)}…` },
          ]}
          data={leaderboard}
          onSelect={(i) => leaderboard[i] && loadProfile(leaderboard[i].proxy_wallet)}
          emptyMessage="Click Refresh to load leaderboard"
        />
      </div>

      {selectedProfile && (
        <div className="w-72 border-l border-border p-4 flex flex-col gap-3">
          <div className="text-xs font-bold text-muted-foreground uppercase">Trader Profile</div>
          <div className="font-mono text-xs break-all">{selectedProfile.address}</div>
          <div className="text-sm">Trades: {selectedProfile.trade_count}</div>
          <div className="text-sm">Category: {selectedProfile.top_category}</div>
          {selectedProfile.win_rate !== null && (
            <div className="text-sm">Win Rate: {((selectedProfile.win_rate ?? 0) * 100).toFixed(1)}%</div>
          )}
          <button
            onClick={() => handleAdd(selectedProfile.address)}
            className="text-sm bg-primary text-primary-foreground px-3 py-2 rounded mt-auto"
          >
            Add to Copy Trading
          </button>
        </div>
      )}
    </div>
  )
}
