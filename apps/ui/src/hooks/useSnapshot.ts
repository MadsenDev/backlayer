import { startTransition, useEffect, useState } from 'react'
import { fetchRuntimeSnapshot } from '../api'
import type { RuntimeSnapshot } from '../types'

export function useSnapshot() {
  const [snapshot, setSnapshot] = useState<RuntimeSnapshot | null>(null)
  const [loading, setLoading] = useState(true)
  const [documentVisible, setDocumentVisible] = useState(
    () => typeof document === 'undefined' ? true : !document.hidden,
  )
  const [disconnectedPollMs, setDisconnectedPollMs] = useState(5000)

  async function refresh(includeAssets = false): Promise<RuntimeSnapshot> {
    const next = await fetchRuntimeSnapshot({ includeAssets })
    startTransition(() => {
      setSnapshot((current) => ({
        ...next,
        assets: includeAssets ? next.assets : current?.assets ?? next.assets,
      }))
      setLoading(false)
    })
    if (next.source === 'tauri_disconnected') {
      setDisconnectedPollMs((current) => Math.min(current * 2, 30000))
    } else {
      setDisconnectedPollMs(5000)
    }
    return next
  }

  useEffect(() => {
    let cancelled = false
    void fetchRuntimeSnapshot({ includeAssets: true }).then((next) => {
      if (cancelled) return
      startTransition(() => {
        setSnapshot(next)
        setLoading(false)
      })
    })
    return () => { cancelled = true }
  }, [])

  useEffect(() => {
    const update = () => setDocumentVisible(!document.hidden)
    document.addEventListener('visibilitychange', update)
    window.addEventListener('focus', update)
    window.addEventListener('blur', update)
    return () => {
      document.removeEventListener('visibilitychange', update)
      window.removeEventListener('focus', update)
      window.removeEventListener('blur', update)
    }
  }, [])

  useEffect(() => {
    if (snapshot?.source !== 'tauri' && snapshot?.source !== 'tauri_disconnected') return
    const intervalMs = snapshot.source === 'tauri_disconnected'
      ? disconnectedPollMs
      : documentVisible ? 5000 : 30000
    const timeout = window.setTimeout(() => void refresh(false).catch(() => {}), intervalMs)
    return () => window.clearTimeout(timeout)
  }, [disconnectedPollMs, documentVisible, snapshot?.source])

  return { snapshot, loading, documentVisible, refresh }
}
