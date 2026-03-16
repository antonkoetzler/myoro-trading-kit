import { useBacktesterStore } from '@/stores/backtester'
import { MetricCard } from '@/components/shared/MetricCard'
import { EquityCurve } from '@/components/charts/EquityCurve'
import { DrawdownChart } from '@/components/charts/DrawdownChart'
import { PnlHistogram } from '@/components/charts/PnlHistogram'
import { FanChart } from '@/components/charts/FanChart'

const METRIC_LABELS: Record<string, string> = {
  total_return: 'Total Return',
  sharpe: 'Sharpe',
  sortino: 'Sortino',
  max_drawdown_pct: 'Max DD%',
  win_rate: 'Win Rate',
  profit_factor: 'Profit Factor',
  trade_count: 'Trades',
  calmar: 'Calmar',
  var_95: 'VaR 95%',
  expectancy: 'Expectancy',
  recovery_factor: 'Recovery Factor',
  avg_win: 'Avg Win',
  avg_loss: 'Avg Loss',
}

function EmptyState({ message }: { message: string }) {
  return (
    <div className="flex h-full items-center justify-center">
      <span className="text-sm text-muted-foreground">{message}</span>
    </div>
  )
}

export function ResultsPanel() {
  const { results } = useBacktesterStore()

  if (!results) {
    return <EmptyState message="Load trades, then press Run to see results" />
  }

  if (results.is_running) {
    return (
      <div className="flex h-full items-center justify-center gap-2">
        <span className="inline-block h-3 w-3 animate-spin rounded-full border-2 border-primary border-t-transparent" />
        <span className="text-sm text-muted-foreground">Running…</span>
      </div>
    )
  }

  // Collect errors from last_error and tool_extra["Error"] key
  const errors: string[] = []
  if (results.last_error) errors.push(results.last_error)
  const extraErrors = results.tool_extra.filter(([k]) => k.toLowerCase() === 'error').map(([, v]) => v)
  errors.push(...extraErrors)

  const hasData = results.equity_curve.length > 0

  const cleanExtra = results.tool_extra.filter(([k]) => k.toLowerCase() !== 'error')

  return (
    <div className="flex h-full flex-col overflow-hidden">
      {/* Error banner */}
      {errors.length > 0 && (
        <div className="flex items-start gap-2 border-b border-destructive/40 bg-destructive/10 px-4 py-2">
          <span className="mt-0.5 text-destructive">⚠</span>
          <div className="flex flex-col gap-0.5">
            {errors.map((e, i) => (
              <span key={i} className="text-xs text-destructive">{e}</span>
            ))}
          </div>
        </div>
      )}

      {!hasData ? (
        <EmptyState message={errors.length > 0 ? 'Fix the error above to see results' : 'No trade data — load trades first'} />
      ) : (
        <div className="flex flex-1 overflow-hidden">
          {/* Metrics panel */}
          <div className="w-56 flex-shrink-0 border-r border-border overflow-auto p-3">
            <div className="text-xs font-bold text-muted-foreground uppercase mb-2">Metrics</div>
            <div className="flex flex-col gap-2">
              {Object.entries(results.metrics).map(([k, v]) => (
                <MetricCard key={k} label={METRIC_LABELS[k] ?? k} value={v} />
              ))}
            </div>
            {cleanExtra.length > 0 && (
              <div className="mt-3">
                <div className="text-xs font-bold text-muted-foreground uppercase mb-1">Analysis</div>
                {cleanExtra.map(([k, v]) => (
                  <div key={k} className="text-xs flex justify-between py-1 border-b border-border/50 gap-2">
                    <span className="text-muted-foreground">{k}</span>
                    <span className="text-right">{v}</span>
                  </div>
                ))}
              </div>
            )}
          </div>

          {/* Charts grid */}
          <div className="flex-1 overflow-auto">
            <div className="grid h-full" style={{ gridTemplateColumns: '1fr 1fr', gridTemplateRows: '50% 50%' }}>
              <div className="border-b border-r border-border p-2">
                <div className="text-xs text-muted-foreground mb-1">Equity Curve</div>
                <EquityCurve data={results.equity_curve} height={160} />
              </div>
              <div className="border-b border-border p-2">
                <div className="text-xs text-muted-foreground mb-1">Drawdown</div>
                <DrawdownChart data={results.drawdown_curve} height={160} />
              </div>
              <div className="border-r border-border p-2">
                <div className="text-xs text-muted-foreground mb-1">P&L Distribution</div>
                <PnlHistogram buckets={results.pnl_buckets} height={160} />
              </div>
              <div className="p-2">
                <div className="text-xs text-muted-foreground mb-1">Monte Carlo Fan</div>
                <FanChart paths={results.mc_paths} height={160} />
              </div>
            </div>
          </div>
        </div>
      )}
    </div>
  )
}
