import { mockIPC } from '@tauri-apps/api/mocks'
import { render } from '@testing-library/react'
import { App } from '../App'
import type { View } from '../api'

export type Call = { cmd: string; args?: Record<string, unknown> }
export type Backend = {
  calls: Call[]
  readonly view: View
  setView: (view: View) => void
}
export type Handler = (args: Record<string, unknown> | undefined, backend: Backend) => unknown

/**
 * Install a fake Rust backend. Every command is recorded; a handler may answer
 * (and update the snapshot) and everything unhandled returns the current view,
 * which is what the real command layer does for the read-mostly commands.
 */
export function installBackend(initial: View, handlers: Record<string, Handler> = {}) {
  const calls: Call[] = []
  let current = initial
  const backend: Backend = {
    calls,
    get view() {
      return current
    },
    setView(next) {
      current = next
    },
  }
  mockIPC((cmd, args) => {
    const payload = args as Record<string, unknown> | undefined
    calls.push({ cmd, args: payload })
    const handler = handlers[cmd]
    if (handler) {
      const result = handler(payload, backend)
      if (result !== undefined) return result
    }
    return current
  })
  return backend
}

/** Render the whole shell against the fake backend (page tests drive it through the sidebar). */
export function renderApp(initial: View, handlers: Record<string, Handler> = {}) {
  const backend = installBackend(initial, handlers)
  return { backend, ...render(<App />) }
}
