import { create } from 'zustand'
import { invoke } from '@tauri-apps/api/core'
import type { BacktesterState, BacktesterResults } from '@/types/backtester'

interface BacktesterStore {
  state: BacktesterState | null
  results: BacktesterResults | null
  selectedStrategyIdx: number
  selectedDataSourceIdx: number
  selectedToolIdx: number
  loading: boolean
  polling: boolean

  refreshState: () => Promise<void>
  refreshResults: () => Promise<void>
  loadTrades: () => Promise<void>
  runTool: () => Promise<void>
  setParam: (toolIdx: number, paramIdx: number, value: number) => Promise<void>
  setSelectedStrategy: (idx: number) => void
  setSelectedDataSource: (idx: number) => void
  setSelectedTool: (idx: number) => void
  startPolling: () => void
  stopPolling: () => void
  _pollInterval: ReturnType<typeof setInterval> | null
}

export const useBacktesterStore = create<BacktesterStore>((set, get) => ({
  state: null,
  results: null,
  selectedStrategyIdx: 0,
  selectedDataSourceIdx: 0,
  selectedToolIdx: 0,
  loading: false,
  polling: false,
  _pollInterval: null,

  refreshState: async () => {
    const data = await invoke<BacktesterState>('get_backtester_state')
    set({ state: data })
  },
  refreshResults: async () => {
    const data = await invoke<BacktesterResults>('get_backtester_results')
    set({ results: data })
    if (!data.is_running) {
      get().stopPolling()
    }
  },
  loadTrades: async () => {
    const { selectedStrategyIdx, selectedDataSourceIdx } = get()
    await invoke('backtester_load_trades', {
      strategyIdx: selectedStrategyIdx,
      dataSourceIdx: selectedDataSourceIdx,
    })
    await get().refreshResults()
  },
  runTool: async () => {
    const { selectedToolIdx } = get()
    await invoke('backtester_run_tool', { toolIdx: selectedToolIdx })
    get().startPolling()
  },
  setParam: async (toolIdx, paramIdx, value) => {
    await invoke('backtester_set_param', { toolIdx, paramIdx, value })
    await get().refreshState()
  },
  setSelectedStrategy: (idx) => set({ selectedStrategyIdx: idx }),
  setSelectedDataSource: (idx) => set({ selectedDataSourceIdx: idx }),
  setSelectedTool: (idx) => set({ selectedToolIdx: idx }),
  startPolling: () => {
    if (get().polling) return
    const id = setInterval(() => get().refreshResults(), 500)
    set({ polling: true, _pollInterval: id })
  },
  stopPolling: () => {
    const id = get()._pollInterval
    if (id) clearInterval(id)
    set({ polling: false, _pollInterval: null })
  },
}))
