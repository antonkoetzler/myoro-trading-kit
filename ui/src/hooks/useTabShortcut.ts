import { useEffect } from 'react'

export interface TabShortcutOptions {
  onUp?: () => void
  onDown?: () => void
  onLeft?: () => void
  onRight?: () => void
  onEnter?: () => void
  onDismiss?: () => void
  onToggle?: () => void
  onAdd?: () => void
  onRefresh?: () => void
  onStartStop?: () => void
  enabled?: boolean
}

export function useTabShortcut(opts: TabShortcutOptions) {
  const {
    onUp, onDown, onLeft, onRight,
    onEnter, onDismiss, onToggle, onAdd, onRefresh, onStartStop,
    enabled = true,
  } = opts

  useEffect(() => {
    if (!enabled) return
    const handler = (e: KeyboardEvent) => {
      const target = e.target as HTMLElement
      if (['INPUT', 'TEXTAREA', 'SELECT'].includes(target.tagName)) return

      switch (e.key) {
        case 'j': case 'ArrowDown': onDown?.(); break
        case 'k': case 'ArrowUp': onUp?.(); break
        case 'h': case 'ArrowLeft': onLeft?.(); break
        case 'l': case 'ArrowRight': onRight?.(); break
        case 'Enter': onEnter?.(); break
        case 'd': onDismiss?.(); break
        case 'e': onToggle?.(); break
        case 'a': onAdd?.(); break
        case 'r': onRefresh?.(); break
        case 's': onStartStop?.(); break
      }
    }
    window.addEventListener('keydown', handler)
    return () => window.removeEventListener('keydown', handler)
  }, [onUp, onDown, onLeft, onRight, onEnter, onDismiss, onToggle, onAdd, onRefresh, onStartStop, enabled])
}
