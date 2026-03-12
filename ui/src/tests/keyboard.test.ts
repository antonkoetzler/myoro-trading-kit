import { describe, it, expect, beforeEach } from 'vitest'
import { useGlobalStore } from '@/stores/global'

describe('global keyboard shortcuts', () => {
  beforeEach(() => {
    useGlobalStore.setState({ activeTab: 0, paletteOpen: false, shortcutsOpen: false })
  })

  it('setActiveTab updates the active tab', () => {
    useGlobalStore.getState().setActiveTab(3)
    expect(useGlobalStore.getState().activeTab).toBe(3)
  })

  it('openPalette sets paletteOpen', () => {
    useGlobalStore.getState().openPalette()
    expect(useGlobalStore.getState().paletteOpen).toBe(true)
  })

  it('openShortcuts sets shortcutsOpen', () => {
    useGlobalStore.getState().openShortcuts()
    expect(useGlobalStore.getState().shortcutsOpen).toBe(true)
  })

  it('closeAllDialogs resets dialog state', () => {
    useGlobalStore.setState({ paletteOpen: true, shortcutsOpen: true })
    useGlobalStore.getState().closeAllDialogs()
    expect(useGlobalStore.getState().paletteOpen).toBe(false)
    expect(useGlobalStore.getState().shortcutsOpen).toBe(false)
  })
})
