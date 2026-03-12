import { invoke } from '@tauri-apps/api/core'

export async function useInvoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  return invoke<T>(cmd, args)
}

export async function callCommand<T>(
  cmd: string,
  args?: Record<string, unknown>,
  onError?: (e: string) => void,
): Promise<T | null> {
  try {
    return await invoke<T>(cmd, args)
  } catch (e) {
    onError?.(String(e))
    return null
  }
}
