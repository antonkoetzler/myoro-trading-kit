import { useSportsStore } from '@/stores/sports'
import { usePoll } from '@/hooks/usePoll'
import { DataTable } from '@/components/shared/DataTable'
import { SignalBadge } from '@/components/shared/SignalBadge'

export function SportsTab() {
  const { state, refresh, toggleStrategy, dismissSignal } = useSportsStore()
  usePoll(refresh, 4000)

  if (!state) return <div className="p-4 text-muted-foreground">Loading…</div>

  return (
    <div className="flex h-full overflow-hidden">
      {/* Left: strategies */}
      <div className="w-56 border-r border-border p-3 flex flex-col gap-2">
        <div className="text-xs font-bold text-muted-foreground uppercase">Strategies</div>
        {state.strategies.map((s, i) => (
          <div key={s.id} className="flex items-center justify-between gap-2">
            <span className="text-xs truncate" title={s.description}>{s.name}</span>
            <button
              onClick={() => toggleStrategy(i, !s.enabled)}
              className={`text-xs px-2 py-0.5 rounded flex-shrink-0 ${s.enabled ? 'bg-success/20 text-success' : 'bg-muted text-muted-foreground'}`}
            >
              {s.enabled ? 'ON' : 'OFF'}
            </button>
          </div>
        ))}
      </div>

      {/* Center: signals */}
      <div className="flex-1 flex flex-col overflow-hidden">
        <div className="text-xs font-bold text-muted-foreground uppercase p-3 border-b border-border">Signals</div>
        <DataTable
          className="flex-1"
          columns={[
            { key: 'home', header: 'Match', render: (r) => `${r.home} vs ${r.away}` },
            { key: 'side', header: 'Side', render: (r) => <SignalBadge side={r.side} /> },
            { key: 'edge_pct', header: 'Edge', render: (r) => `${(r.edge_pct * 100).toFixed(1)}%` },
            { key: 'strategy_id', header: 'Strategy' },
            { key: 'market_id', header: '', render: (r) => (
              <button onClick={() => dismissSignal(r.market_id)} className="text-xs text-muted-foreground">Dismiss</button>
            )},
          ]}
          data={state.signals.filter((s) => s.status === 'pending')}
        />
      </div>

      {/* Right: fixtures */}
      <div className="w-72 border-l border-border overflow-auto">
        <div className="text-xs font-bold text-muted-foreground uppercase p-3 border-b border-border">Fixtures</div>
        <DataTable
          columns={[
            { key: 'home', header: 'Match', render: (r) => `${r.home} vs ${r.away}` },
            { key: 'date', header: 'Date' },
          ]}
          data={state.fixtures}
        />
      </div>
    </div>
  )
}
