import { useEffect } from 'react'
import { useBacktesterStore } from '@/stores/backtester'
import { StrategySelector } from './StrategySelector'
import { DataSourceConfig } from './DataSourceConfig'
import { ToolSelector } from './ToolSelector'
import { ParamEditor } from './ParamEditor'
import { ResultsPanel } from './ResultsPanel'

export function BacktesterTab() {
  const { refreshState } = useBacktesterStore()
  useEffect(() => { refreshState() }, [refreshState])

  return (
    <div className="h-full grid overflow-hidden" style={{ gridTemplateRows: '40% 15% 45%' }}>
      {/* Top row: 3 panels */}
      <div className="grid border-b border-border overflow-hidden" style={{ gridTemplateColumns: '1fr 1fr 1fr' }}>
        <StrategySelector />
        <DataSourceConfig />
        <ToolSelector />
      </div>
      {/* Param row */}
      <div className="border-b border-border overflow-auto">
        <ParamEditor />
      </div>
      {/* Results */}
      <div className="overflow-hidden">
        <ResultsPanel />
      </div>
    </div>
  )
}
