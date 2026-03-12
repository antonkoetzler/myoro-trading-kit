import { useGlobalStore } from '@/stores/global'

export function Header() {
  const { globalStats, config } = useGlobalStore()

  const bankroll = globalStats?.bankroll ?? config?.paper_bankroll ?? null
  const pnl = globalStats?.pnl ?? 0
  const mode = config?.execution_mode ?? 'paper'
  const currency = config?.pnl_currency ?? 'USD'

  return (
    <header className="flex items-center justify-between px-4 py-2 border-b border-border bg-card">
      <div className="flex items-center gap-3">
        <span className="font-bold text-primary text-lg">Myoro</span>
        <span className="text-muted-foreground text-sm">Trading Kit</span>
      </div>
      <div className="flex items-center gap-6 text-sm">
        {bankroll !== null && (
          <span className="text-foreground">
            <span className="text-muted-foreground mr-1">Bankroll:</span>
            {currency} {bankroll.toFixed(2)}
          </span>
        )}
        <span className={pnl >= 0 ? 'text-success' : 'text-destructive'}>
          <span className="text-muted-foreground mr-1">P&L:</span>
          {pnl >= 0 ? '+' : ''}{pnl.toFixed(2)}
        </span>
        <span
          className={`px-2 py-0.5 rounded text-xs font-medium uppercase ${
            mode === 'live'
              ? 'bg-destructive text-destructive-foreground'
              : 'bg-secondary text-secondary-foreground'
          }`}
        >
          {mode}
        </span>
      </div>
    </header>
  )
}
