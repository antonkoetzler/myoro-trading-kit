import { useGlobalStore } from '@/stores/global'

export function StatusBar() {
  const { globalStats } = useGlobalStore()

  if (!globalStats?.circuit_breaker_active) {
    return (
      <footer className="px-4 py-1 text-xs text-muted-foreground border-t border-border bg-card flex justify-between">
        <span>Ready</span>
        <span>? = shortcuts | , = settings | Cmd+K = palette</span>
      </footer>
    )
  }

  return (
    <footer className="px-4 py-1 text-xs border-t border-border bg-destructive text-destructive-foreground flex items-center gap-2">
      <span className="font-bold">CIRCUIT BREAKER ACTIVE</span>
      <span>— all trading suspended. Daily loss limit reached.</span>
    </footer>
  )
}
