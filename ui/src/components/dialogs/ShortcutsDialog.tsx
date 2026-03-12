import { useGlobalStore } from '@/stores/global'

const SHORTCUTS = [
  { key: 'Cmd+K', desc: 'Command palette' },
  { key: '?', desc: 'Show shortcuts' },
  { key: ',', desc: 'Settings' },
  { key: '0–8', desc: 'Switch tabs' },
  { key: 'Esc', desc: 'Close dialog' },
  { key: 'j / ↓', desc: 'Move selection down' },
  { key: 'k / ↑', desc: 'Move selection up' },
  { key: 'h / ←', desc: 'Move left / previous pane' },
  { key: 'l / →', desc: 'Move right / next pane' },
  { key: 'Enter', desc: 'Execute / confirm' },
  { key: 'd', desc: 'Dismiss signal' },
  { key: 'e', desc: 'Toggle enable/disable strategy' },
  { key: 'a', desc: 'Add trader / item' },
  { key: 'r', desc: 'Refresh tab data' },
  { key: 's', desc: 'Start / stop (copy, MM)' },
]

export function ShortcutsDialog() {
  const { shortcutsOpen, closeAllDialogs } = useGlobalStore()
  if (!shortcutsOpen) return null

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-background/80 backdrop-blur-sm" onClick={closeAllDialogs}>
      <div className="bg-card border border-border rounded-lg shadow-xl w-full max-w-md p-6" onClick={(e) => e.stopPropagation()}>
        <div className="text-sm font-bold mb-4">Keyboard Shortcuts</div>
        <div className="grid gap-2">
          {SHORTCUTS.map(({ key, desc }) => (
            <div key={key} className="flex items-center justify-between text-sm">
              <span className="text-muted-foreground">{desc}</span>
              <kbd className="text-xs bg-muted px-2 py-0.5 rounded font-mono">{key}</kbd>
            </div>
          ))}
        </div>
        <button onClick={closeAllDialogs} className="mt-4 text-xs text-muted-foreground">Press Esc to close</button>
      </div>
    </div>
  )
}
