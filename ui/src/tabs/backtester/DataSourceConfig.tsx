import { useBacktesterStore } from '@/stores/backtester'

export function DataSourceConfig() {
  const { state, selectedDataSourceIdx, setSelectedDataSource } = useBacktesterStore()

  return (
    <div className="flex flex-col border-r border-border overflow-hidden p-3 gap-2">
      <div className="text-xs font-bold text-muted-foreground uppercase">Data Source</div>
      <div className="flex-1 overflow-auto">
        {state?.data_sources.map((s, i) => (
          <button
            key={s.id}
            onClick={() => setSelectedDataSource(i)}
            className={`w-full text-left text-xs px-2 py-1.5 rounded mb-1 ${
              selectedDataSourceIdx === i ? 'bg-primary text-primary-foreground' : 'hover:bg-muted'
            }`}
          >
            <div className="font-medium">{s.name}</div>
            <div className="text-muted-foreground">{s.domain}</div>
          </button>
        ))}
      </div>
    </div>
  )
}
