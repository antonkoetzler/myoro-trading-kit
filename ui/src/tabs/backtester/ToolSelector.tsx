import { useBacktesterStore } from '@/stores/backtester'

export function ToolSelector() {
  const { state, selectedToolIdx, setSelectedTool, runTool } = useBacktesterStore()

  const tiers = [
    { label: 'Tier 0 — Permutation', range: [0, 2] },
    { label: 'Tier 1 — Core', range: [3, 6] },
    { label: 'Tier 2 — Robustness', range: [7, 10] },
    { label: 'Tier 3 — Risk', range: [11, 14] },
    { label: 'Tier 4 — Simulation', range: [15, 18] },
  ]

  return (
    <div className="flex flex-col overflow-hidden p-3 gap-2">
      <div className="text-xs font-bold text-muted-foreground uppercase">Tool</div>
      <div className="flex-1 overflow-auto">
        {tiers.map((tier) => (
          <div key={tier.label} className="mb-2">
            <div className="text-xs text-muted-foreground mb-1">{tier.label}</div>
            {state?.tools.slice(tier.range[0], tier.range[1] + 1).map((tool) => (
              <button
                key={tool.idx}
                title={tool.tooltip}
                onClick={() => setSelectedTool(tool.idx)}
                className={`w-full text-left text-xs px-2 py-1 rounded mb-0.5 ${
                  selectedToolIdx === tool.idx ? 'bg-primary text-primary-foreground' : 'hover:bg-muted'
                }`}
              >
                {tool.name}
              </button>
            ))}
          </div>
        ))}
      </div>
      <button
        onClick={runTool}
        className="text-xs bg-primary text-primary-foreground px-3 py-2 rounded font-medium"
      >
        Run
      </button>
    </div>
  )
}
