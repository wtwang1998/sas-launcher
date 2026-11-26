import { CircleAlert } from 'lucide-react'
import { useId, useLayoutEffect, useRef, useState } from 'react'
import type { Language } from '../api'
import type { Notice } from '../lib/notice'
import type { Translator } from '../i18n'
import { NoticeBanner } from './Notice'

/**
 * `true` dismisses the dialog; `false` keeps it open, which is what an action
 * that reports into the dialog's own notice needs.
 */
export type ConfirmAction = () => boolean | Promise<boolean>

export type Confirm = (title: string, message: string, label: string, action: ConfirmAction) => void
type Request = {
  title: string
  message: string
  label: string
  action: ConfirmAction
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
export function useConfirmation({
  t,
  language,
  notice,
  onDismissNotice,
}: {
  t: Translator
  language: Language
  notice: Notice | null
  onDismissNotice: () => void
}) {
  const request = useRef<Request | null>(null)
  const dialog = useRef<HTMLDialogElement>(null)
  const cancel = useRef<HTMLButtonElement>(null)
  const [active, setActive] = useState<Request | null>(null)
  const [running, setRunning] = useState(false)
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

  function dismiss() {
    const pending = request.current
    request.current = null
    if (dialog.current) closeModal(dialog.current)
    setActive(null)
    pending?.trigger?.focus({ preventScroll: true })
  }

  async function accept() {
    const pending = request.current
    if (!pending || running) return
    setRunning(true)
    let keep = false
    try {
      keep = (await pending.action()) === false
    } finally {
      setRunning(false)
    }
    if (keep) return
    dismiss()
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
          if (!running) dismiss()
        }}
      >
        <div className="app-dialog-content">
          <div className="app-dialog-heading">
            <CircleAlert size={20} aria-hidden="true" />
            <h2 id={labelId}>{active.title}</h2>
          </div>
          <p id={detailId}>{active.message}</p>
          {notice?.target === 'dialog' && (
            <NoticeBanner notice={notice} language={language} t={t} onDismiss={onDismissNotice} />
          )}
          <div className="form-actions">
            <button
              type="button"
              className="button"
              ref={cancel}
              disabled={running}
              onClick={dismiss}
            >
              {t('common.cancel')}
            </button>
            <button
              type="button"
              className="button danger-primary"
              disabled={running}
              onClick={() => void accept()}
            >
              {active.label}
            </button>
          </div>
        </div>
      </dialog>
    ),
  }
}
