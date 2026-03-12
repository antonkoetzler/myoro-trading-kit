import { useBacktesterStore } from '@/stores/backtester'

export function ParamEditor() {
  const { state, selectedToolIdx, setParam } = useBacktesterStore()
  const tool = state?.tools[selectedToolIdx]

  if (!tool || tool.params.length === 0) {
    return (
      <div className="p-3 text-xs text-muted-foreground">No parameters for this tool</div>
    )
  }

  return (
    <div className="p-3 flex flex-wrap gap-4">
      {tool.params.map((param, i) => (
        <div key={param.name} className="flex flex-col gap-1 min-w-32">
          <label className="text-xs text-muted-foreground">{param.name}</label>
          <input
            type="range"
            min={param.min}
            max={param.max}
            step={param.step}
            value={param.value}
            onChange={(e) => setParam(selectedToolIdx, i, parseFloat(e.target.value))}
            className="w-full"
          />
          <div className="text-xs tabular-nums">{param.value.toFixed(param.step < 1 ? 2 : 0)}</div>
        </div>
      ))}
    </div>
  )
}
