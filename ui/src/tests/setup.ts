import '@testing-library/jest-dom'

// Mock Tauri invoke globally
vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn().mockResolvedValue(null),
}))
