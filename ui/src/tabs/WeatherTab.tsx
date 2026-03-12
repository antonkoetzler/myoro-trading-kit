import { useWeatherStore } from '@/stores/weather'
import { usePoll } from '@/hooks/usePoll'
import { SignalBadge } from '@/components/shared/SignalBadge'

export function WeatherTab() {
  const { state, refresh, toggleStrategy } = useWeatherStore()
  usePoll(refresh, 4000)

  if (!state) return <div className="p-4 text-muted-foreground">Loading…</div>

  return (
    <div className="flex h-full overflow-hidden">
      {/* Left: city forecasts */}
      <div className="w-64 border-r border-border p-3 flex flex-col gap-3">
        <div className="text-xs font-bold text-muted-foreground uppercase">Forecasts</div>
        {state.city_forecasts.map((c) => (
          <div key={c.city} className="bg-card rounded border border-border p-2">
            <div className="font-medium text-sm">{c.city}</div>
            <div className="text-xs text-muted-foreground mt-1">
              {c.today_max_c !== null && <span>Max: {c.today_max_c.toFixed(1)}°C </span>}
              {c.today_min_c !== null && <span>Min: {c.today_min_c.toFixed(1)}°C</span>}
            </div>
            {c.market_implied !== null && (
              <div className="text-xs mt-1">Market: {(c.market_implied * 100).toFixed(0)}%</div>
            )}
          </div>
        ))}

        <div className="text-xs font-bold text-muted-foreground uppercase mt-2">Strategies</div>
        {state.strategies.map((s, i) => (
          <div key={s.id} className="flex items-center justify-between">
            <span className="text-sm">{s.name}</span>
            <button
              onClick={() => toggleStrategy(i, !s.enabled)}
              className={`text-xs px-2 py-0.5 rounded ${s.enabled ? 'bg-success/20 text-success' : 'bg-muted text-muted-foreground'}`}
            >
              {s.enabled ? 'ON' : 'OFF'}
            </button>
          </div>
        ))}
      </div>

      {/* Right: signals */}
      <div className="flex-1 overflow-auto p-3">
        <div className="text-xs font-bold text-muted-foreground uppercase mb-3">Signals</div>
        {state.signals.filter((s) => s.status === 'pending').length === 0 ? (
          <div className="text-muted-foreground text-sm">No pending signals</div>
        ) : (
          state.signals.filter((s) => s.status === 'pending').map((sig) => (
            <div key={sig.market_id} className="bg-card rounded border border-border p-3 mb-2">
              <div className="flex items-center justify-between">
                <span className="font-medium text-sm">{sig.city}</span>
                <SignalBadge side={sig.side} />
              </div>
              <div className="text-xs text-muted-foreground mt-1">{sig.label}</div>
              <div className="text-xs mt-1">Edge: {(sig.edge_pct * 100).toFixed(1)}% | Kelly: ${sig.kelly_size.toFixed(2)}</div>
            </div>
          ))
        )}
      </div>
    </div>
  )
}
