import { useGlobalStore } from '@/stores/global'
import { cn } from '@/lib/utils'

const TABS = [
  'Crypto', 'Sports', 'Weather', 'Copy', 'Portfolio', 'Signals', 'Discover', 'Backtester', 'Market Making',
]

export function TabBar() {
  const { activeTab, setActiveTab } = useGlobalStore()

  return (
    <nav className="flex border-b border-border bg-card px-2">
      {TABS.map((name, i) => (
        <button
          key={name}
          onClick={() => setActiveTab(i)}
          className={cn(
            'px-4 py-2 text-sm font-medium transition-colors border-b-2 -mb-px',
            activeTab === i
              ? 'border-primary text-primary'
              : 'border-transparent text-muted-foreground hover:text-foreground',
          )}
        >
          <span className="text-muted-foreground text-xs mr-1">{i}</span>
          {name}
        </button>
      ))}
    </nav>
  )
}
