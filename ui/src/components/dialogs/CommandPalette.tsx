import { useGlobalStore } from '@/stores/global'

export function CommandPalette() {
  const { paletteOpen, closeAllDialogs, setActiveTab } = useGlobalStore()

  if (!paletteOpen) return null

  const actions = [
    { label: 'Crypto Tab', shortcut: '0', action: () => { setActiveTab(0); closeAllDialogs() } },
    { label: 'Sports Tab', shortcut: '1', action: () => { setActiveTab(1); closeAllDialogs() } },
    { label: 'Weather Tab', shortcut: '2', action: () => { setActiveTab(2); closeAllDialogs() } },
    { label: 'Copy Tab', shortcut: '3', action: () => { setActiveTab(3); closeAllDialogs() } },
    { label: 'Portfolio Tab', shortcut: '4', action: () => { setActiveTab(4); closeAllDialogs() } },
    { label: 'Signals Tab', shortcut: '5', action: () => { setActiveTab(5); closeAllDialogs() } },
    { label: 'Discover Tab', shortcut: '6', action: () => { setActiveTab(6); closeAllDialogs() } },
    { label: 'Backtester Tab', shortcut: '7', action: () => { setActiveTab(7); closeAllDialogs() } },
    { label: 'Market Making Tab', shortcut: '8', action: () => { setActiveTab(8); closeAllDialogs() } },
    { label: 'Open Settings', shortcut: ',', action: () => { closeAllDialogs(); useGlobalStore.getState().openSettings() } },
    { label: 'Close', shortcut: 'Esc', action: closeAllDialogs },
  ]

  return (
    <div className="fixed inset-0 z-50 flex items-start justify-center pt-24 bg-background/80 backdrop-blur-sm">
      <div className="bg-card border border-border rounded-lg shadow-xl w-full max-w-lg overflow-hidden">
        <div className="p-3 border-b border-border text-xs text-muted-foreground">Command Palette</div>
        <div className="max-h-80 overflow-auto">
          {actions.map((a) => (
            <button
              key={a.label}
              onClick={a.action}
              className="w-full flex items-center justify-between px-4 py-2.5 text-sm hover:bg-muted transition-colors"
            >
              <span>{a.label}</span>
              <kbd className="text-xs text-muted-foreground bg-muted px-1.5 py-0.5 rounded">{a.shortcut}</kbd>
            </button>
          ))}
        </div>
      </div>
    </div>
  )
}
