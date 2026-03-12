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

export function ResultsPanel() {
  const { results } = useBacktesterStore()

  if (!results) {
    return <div className="p-4 text-muted-foreground text-sm">Run a tool to see results</div>
  }

  if (results.is_running) {
    return <div className="p-4 text-muted-foreground text-sm">Running…</div>
  }

  return (
    <div className="flex h-full overflow-hidden">
      {/* Metrics table */}
      <div className="w-64 border-r border-border p-3 overflow-auto">
        <div className="text-xs font-bold text-muted-foreground uppercase mb-2">Metrics</div>
        <div className="flex flex-col gap-2">
          {Object.entries(results.metrics).map(([k, v]) => (
            <MetricCard key={k} label={METRIC_LABELS[k] ?? k} value={v} />
          ))}
        </div>
        {results.tool_extra.length > 0 && (
          <div className="mt-3">
            <div className="text-xs font-bold text-muted-foreground uppercase mb-1">Analysis</div>
            {results.tool_extra.map(([k, v]) => (
              <div key={k} className="text-xs flex justify-between py-1 border-b border-border/50">
                <span className="text-muted-foreground">{k}</span>
                <span>{v}</span>
              </div>
            ))}
          </div>
        )}
      </div>

      {/* Charts */}
      <div className="flex-1 overflow-auto">
        <div className="grid" style={{ gridTemplateColumns: '1fr 1fr', gridTemplateRows: '50% 50%' }}>
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
  )
}
