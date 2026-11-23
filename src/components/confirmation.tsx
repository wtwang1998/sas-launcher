import { CircleAlert } from 'lucide-react'
import { useId, useLayoutEffect, useRef, useState } from 'react'
import type { Translator } from '../i18n'

export type Confirm = (
  title: string,
  message: string,
  label: string,
  action: () => void | Promise<void>,
) => void
type Request = {
  title: string
  message: string
  label: string
  action: () => void | Promise<void>
  trigger: HTMLElement | null
}

/** jsdom does not always implement the native dialog methods; fall back to the attribute. */
function showModal(dialog: HTMLDialogElement) {
  try {
    if (typeof dialog.showModal === 'function') {
      dialog.showModal()
      return
    }
  } catch {
    // fall through to the attribute form
  }
  dialog.setAttribute('open', '')
}

function closeModal(dialog: HTMLDialogElement) {
  try {
    if (typeof dialog.close === 'function') {
      dialog.close()
      return
    }
  } catch {
    // fall through to the attribute form
  }
  dialog.removeAttribute('open')
}

/** Native-<dialog> confirmation, salvaged from v0.1 and hardened for the test renderer. */
export function useConfirmation(t: Translator) {
  const request = useRef<Request | null>(null)
  const dialog = useRef<HTMLDialogElement>(null)
  const cancel = useRef<HTMLButtonElement>(null)
  const [active, setActive] = useState<Request | null>(null)
  const labelId = useId()
  const detailId = useId()

  useLayoutEffect(() => {
    if (active && dialog.current) {
      showModal(dialog.current)
      cancel.current?.focus({ preventScroll: true })
    }
  }, [active])

  const confirm: Confirm = (title, message, label, action) => {
    if (request.current) return
    const trigger = document.activeElement instanceof HTMLElement ? document.activeElement : null
    const pending = { title, message, label, action, trigger }
    request.current = pending
    setActive(pending)
  }

  function finish(accepted: boolean) {
    const pending = request.current
    if (!pending) return
    request.current = null
    if (dialog.current) closeModal(dialog.current)
    setActive(null)
    if (!accepted) {
      pending.trigger?.focus({ preventScroll: true })
      return
    }

    void (async () => {
      try {
        await pending.action()
      } finally {
        window.requestAnimationFrame(() => {
          if (document.activeElement === document.body && pending.trigger?.isConnected) {
            pending.trigger.focus({ preventScroll: true })
          }
        })
      }
    })()
  }

  return {
    confirm,
    confirmation: active && (
      <dialog
        className="app-dialog"
        ref={dialog}
        aria-labelledby={labelId}
        aria-describedby={detailId}
        onCancel={(event) => {
          event.preventDefault()
          finish(false)
        }}
      >
        <div className="app-dialog-content">
          <div className="app-dialog-heading">
            <CircleAlert size={20} aria-hidden="true" />
            <h2 id={labelId}>{active.title}</h2>
          </div>
          <p id={detailId}>{active.message}</p>
          <div className="form-actions">
            <button type="button" className="button" ref={cancel} onClick={() => finish(false)}>
              {t('cancel')}
            </button>
            <button type="button" className="button danger-primary" onClick={() => finish(true)}>
              {active.label}
            </button>
          </div>
        </div>
      </dialog>
    ),
  }
}
