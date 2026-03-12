import { useSignalsStore } from '@/stores/signals'
import { usePoll } from '@/hooks/usePoll'
import { SignalBadge } from '@/components/shared/SignalBadge'
import { DataTable } from '@/components/shared/DataTable'

export function SignalsTab() {
  const { signals, refresh } = useSignalsStore()
  usePoll(refresh, 4000)

  const all = [
    ...signals.crypto.map((s) => ({ ...s, domain: 'crypto', label: s.label })),
    ...signals.sports.map((s) => ({ ...s, domain: 'sports', label: `${s.home} vs ${s.away}` })),
    ...signals.weather.map((s) => ({ ...s, domain: 'weather', label: s.label })),
  ]

  return (
    <div className="flex flex-col h-full overflow-hidden">
      <div className="p-3 border-b border-border text-sm text-muted-foreground">
        {all.length} pending signals across all domains
      </div>
      <DataTable
        className="flex-1"
        columns={[
          { key: 'domain', header: 'Domain', render: (r) => <span className="text-xs uppercase text-muted-foreground">{r.domain}</span> },
          { key: 'label', header: 'Market' },
          { key: 'side', header: 'Side', render: (r) => <SignalBadge side={r.side} /> },
          { key: 'edge_pct', header: 'Edge', render: (r) => `${(r.edge_pct * 100).toFixed(1)}%` },
          { key: 'kelly_size', header: 'Kelly', render: (r) => `$${r.kelly_size.toFixed(2)}` },
          { key: 'strategy_id', header: 'Strategy' },
        ]}
        data={all}
        emptyMessage="No pending signals"
      />
    </div>
  )
}
