import { useCryptoStore } from '@/stores/crypto'
import { usePoll } from '@/hooks/usePoll'
import { DataTable } from '@/components/shared/DataTable'
import { SignalBadge } from '@/components/shared/SignalBadge'

export function CryptoTab() {
  const { state, refresh, toggleStrategy, dismissSignal } = useCryptoStore()
  usePoll(refresh, 4000)

  if (!state) return <div className="p-4 text-muted-foreground">Loading…</div>

  return (
    <div className="flex h-full gap-0 overflow-hidden">
      {/* Left: strategies + price */}
      <div className="w-64 border-r border-border flex flex-col p-3 gap-3">
        <div className="text-xs font-bold text-muted-foreground uppercase">Price</div>
        <div className="text-sm font-mono">{state.btc_usdt || '—'}</div>
        <div className="text-xs font-bold text-muted-foreground uppercase mt-2">Strategies</div>
        {state.strategies.map((s, i) => (
          <div key={s.id} className="flex items-center justify-between">
            <span className="text-sm">{s.name}</span>
            <button
              onClick={() => toggleStrategy(i, !s.enabled)}
              className={`text-xs px-2 py-0.5 rounded ${s.enabled ? 'bg-success/20 text-success' : 'bg-muted text-muted-foreground'}`}
            >
              {s.enabled ? 'ON' : 'OFF'}
            </button>
          </div>
        ))}
      </div>

      {/* Center: signals */}
      <div className="flex-1 overflow-auto">
        <DataTable
          columns={[
            { key: 'label', header: 'Market' },
            {
              key: 'side',
              header: 'Side',
              render: (row) => <SignalBadge side={row.side} status={row.status} />,
            },
            { key: 'edge_pct', header: 'Edge', render: (row) => `${(row.edge_pct * 100).toFixed(1)}%` },
            { key: 'kelly_size', header: 'Kelly', render: (row) => `$${row.kelly_size.toFixed(2)}` },
            {
              key: 'market_id',
              header: '',
              render: (row) => (
                <button
                  onClick={() => dismissSignal(row.market_id)}
                  className="text-xs text-muted-foreground hover:text-foreground"
                >
                  Dismiss
                </button>
              ),
            },
          ]}
          data={state.signals.filter((s) => s.status === 'pending')}
          emptyMessage="No pending signals"
        />
      </div>

      {/* Right: markets */}
      <div className="w-72 border-l border-border overflow-auto">
        <DataTable
          columns={[
            { key: 'title', header: 'Market' },
            { key: 'best_bid', header: 'Bid', render: (r) => r.best_bid.toFixed(3) },
            { key: 'best_ask', header: 'Ask', render: (r) => r.best_ask.toFixed(3) },
          ]}
          data={state.markets}
        />
      </div>
    </div>
  )
}
