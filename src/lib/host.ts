import { listen } from '@tauri-apps/api/event'

/**
 * Subscribes to an event the Rust shell emits.
 *
 * The same name is also accepted as a plain DOM event, which keeps the shell
 * flows testable (and previewable in a browser) without a Tauri window.
 */
export function onHostEvent(name: string, handler: () => void): () => void {
  let stop: (() => void) | null = null
  let cancelled = false
  void (async () => {
    try {
      const unlisten = await listen(name, () => handler())
      if (cancelled) unlisten()
      else stop = unlisten
    } catch {
      // Not running inside a Tauri window (unit tests, browser preview).
    }
  })()

  const local = () => handler()
  document.addEventListener(name, local)

  return () => {
    cancelled = true
    stop?.()
    document.removeEventListener(name, local)
  }
}
