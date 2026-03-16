import { useBacktesterStore } from '@/stores/backtester'

export function StrategySelector() {
  const { state, selectedStrategyIdx, setSelectedStrategy, loadTrades } = useBacktesterStore()

  return (
    <div className="flex flex-col border-r border-border overflow-hidden p-3 gap-2">
      <div className="text-xs font-bold text-muted-foreground uppercase">Strategy</div>
      <div className="flex-1 overflow-auto">
        {state?.strategies.map((s, i) => (
          <button
            key={s.id}
            onClick={() => setSelectedStrategy(i)}
            className={`w-full text-left text-xs px-2 py-1.5 rounded mb-1 ${
              selectedStrategyIdx === i ? 'bg-primary text-primary-foreground' : 'hover:bg-muted'
            }`}
          >
            <div className="font-medium">{s.name}</div>
            <div className={selectedStrategyIdx === i ? 'text-primary-foreground/70' : 'text-muted-foreground'}>{s.domain}</div>
          </button>
        ))}
      </div>
      <button
        onClick={loadTrades}
        className="text-xs bg-secondary text-secondary-foreground px-3 py-1.5 rounded"
      >
        Load Trades
      </button>
    </div>
  )
}
