import { useEffect } from 'react'
import { useGlobalStore } from '@/stores/global'

const DIALOG_TAGS = new Set(['INPUT', 'TEXTAREA', 'SELECT'])

export function useKeyboard() {
  const { setActiveTab, openPalette, openShortcuts, openSettings, closeAllDialogs } =
    useGlobalStore()

  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      const target = e.target as HTMLElement
      if (DIALOG_TAGS.has(target.tagName)) return

      // Cmd/Ctrl+K → command palette
      if ((e.metaKey || e.ctrlKey) && e.key === 'k') {
        e.preventDefault()
        openPalette()
        return
      }

      // Escape → close dialogs
      if (e.key === 'Escape') {
        closeAllDialogs()
        return
      }

      // ? → shortcuts overlay
      if (e.key === '?' && !e.metaKey && !e.ctrlKey) {
        openShortcuts()
        return
      }

      // , → settings
      if (e.key === ',' && !e.metaKey && !e.ctrlKey) {
        openSettings()
        return
      }

      // 0-8 → switch tabs
      if (!e.metaKey && !e.ctrlKey && !e.altKey) {
        const num = parseInt(e.key)
        if (!isNaN(num) && num >= 0 && num <= 8) {
          setActiveTab(num)
        }
      }
    }

    window.addEventListener('keydown', handler)
    return () => window.removeEventListener('keydown', handler)
  }, [setActiveTab, openPalette, openShortcuts, openSettings, closeAllDialogs])
}
