import { usePortfolioStore } from '@/stores/portfolio'
import { usePoll } from '@/hooks/usePoll'
import { DataTable } from '@/components/shared/DataTable'
import { MetricCard } from '@/components/shared/MetricCard'

export function PortfolioTab() {
  const { state, refresh } = usePortfolioStore()
  usePoll(refresh, 4000)

  if (!state) return <div className="p-4 text-muted-foreground">Loading…</div>

  return (
    <div className="flex flex-col h-full overflow-hidden">
      {/* Domain P&L cards */}
      <div className="flex gap-3 p-3 border-b border-border">
        <MetricCard label="Total P&L" value={state.total_pnl} />
        {state.domain_pnl.map((d) => (
          <MetricCard key={d.domain} label={`${d.domain} P&L`} value={d.alltime_pnl} />
        ))}
      </div>

      <div className="flex flex-1 overflow-hidden">
        {/* Open positions */}
        <div className="flex-1 flex flex-col overflow-hidden border-r border-border">
          <div className="text-xs font-bold text-muted-foreground uppercase p-3 border-b border-border">
            Open Positions ({state.open_positions.length})
          </div>
          <DataTable
            columns={[
              { key: 'market_id', header: 'Market' },
              { key: 'outcome', header: 'Outcome' },
              { key: 'size', header: 'Size', render: (r) => r.size.toFixed(2) },
              { key: 'avg_price', header: 'Avg Price', render: (r) => r.avg_price.toFixed(3) },
              { key: 'current_value', header: 'Value', render: (r) => `$${r.current_value.toFixed(2)}` },
            ]}
            data={state.open_positions}
            emptyMessage="No open positions"
          />
        </div>

        {/* Trade history */}
        <div className="flex-1 flex flex-col overflow-hidden">
          <div className="text-xs font-bold text-muted-foreground uppercase p-3 border-b border-border">
            Trade History ({state.trade_history.length})
          </div>
          <DataTable
            columns={[
              { key: 'timestamp', header: 'Time', render: (r) => r.timestamp.slice(0, 16) },
              { key: 'domain', header: 'Domain' },
              { key: 'side', header: 'Side' },
              { key: 'size', header: 'Size', render: (r) => r.size.toFixed(2) },
              { key: 'price', header: 'Price', render: (r) => r.price.toFixed(3) },
            ]}
            data={state.trade_history}
          />
        </div>
      </div>
    </div>
  )
}
