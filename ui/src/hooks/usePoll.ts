import { useEffect, useRef } from 'react'

export function usePoll(fn: () => void | Promise<void>, intervalMs: number) {
  const fnRef = useRef(fn)
  fnRef.current = fn

  useEffect(() => {
    fnRef.current()
    const id = setInterval(() => fnRef.current(), intervalMs)
    return () => clearInterval(id)
  }, [intervalMs])
}
