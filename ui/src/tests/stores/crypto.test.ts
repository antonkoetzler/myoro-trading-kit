import { describe, it, expect, beforeEach, vi } from 'vitest'
import { invoke } from '@tauri-apps/api/core'
import { useCryptoStore } from '@/stores/crypto'

vi.mocked(invoke).mockResolvedValue({
  btc_usdt: 'BTC/USDT 60000',
  events: [],
  strategies: [],
  signals: [],
  markets: [],
  binance_lag_confidence: [],
  logical_arb_confidence: [],
  logs: [],
})

describe('CryptoStore', () => {
  beforeEach(() => {
    useCryptoStore.setState({ state: null, loading: false })
  })

  it('refresh populates state', async () => {
    await useCryptoStore.getState().refresh()
    expect(useCryptoStore.getState().state?.btc_usdt).toBe('BTC/USDT 60000')
  })

  it('toggleStrategy calls invoke with correct args', async () => {
    await useCryptoStore.getState().toggleStrategy(0, false)
    expect(invoke).toHaveBeenCalledWith('toggle_crypto_strategy', { idx: 0, enabled: false })
  })
})
