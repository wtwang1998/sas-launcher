import { Activity, Eraser } from 'lucide-react'
import { useState } from 'react'
import type { LogSeverity, SessionLog, View } from '../api'
import { NoticeBanner } from '../components/Notice'
import { SessionRow } from '../components/SessionRow'
import type { Notice } from '../lib/notice'
import { useT } from '../i18n/provider'

export type DiagnosticsProps = {
  view: View
  busy: boolean
  notice: Notice | null
  onDismissNotice: () => void
  onClear: () => Promise<void>
  onOpenLog: (id: string) => void
  onOpenSasuser: (id: string) => void
  onLoadLog: (id: string) => Promise<SessionLog>
}

export function Diagnostics({
  view,
  busy,
  notice,
  onDismissNotice,
  onClear,
  onOpenLog,
  onOpenSasuser,
  onLoadLog,
}: DiagnosticsProps) {
  const t = useT()
  const [expanded, setExpanded] = useState<ReadonlySet<string>>(() => new Set())
  const [logs, setLogs] = useState<Readonly<Record<string, SessionLog | 'loading'>>>({})
  const [requested, setRequested] = useState<Readonly<Record<string, LogSeverity>>>({})

  // The chips are switches: pressing the open one closes the list.
  async function showLog(id: string, severity: LogSeverity) {
    if (requested[id] === severity) {
      closeLog(id)
      return
    }
    setRequested((current) => ({ ...current, [id]: severity }))
    if (logs[id] !== undefined && logs[id] !== 'loading') return
    setLogs((current) => ({ ...current, [id]: 'loading' }))
    try {
      const report = await onLoadLog(id)
      setLogs((current) => ({ ...current, [id]: report }))
    } catch {
      setLogs((current) => {
        const next = { ...current }
        delete next[id]
        return next
      })
    }
  }

  // Dropping the report is deliberate: it is a snapshot of when the chip was
  // pressed, and sas.log keeps growing underneath it.
  function closeLog(id: string) {
    setRequested((current) => {
      const next = { ...current }
      delete next[id]
      return next
    })
    setLogs((current) => {
      const next = { ...current }
      delete next[id]
      return next
    })
  }

  function toggle(id: string) {
    if (expanded.has(id)) closeLog(id)
    setExpanded((current) => {
      const next = new Set(current)
      if (next.has(id)) next.delete(id)
      else next.add(id)
      return next
    })
  }

  return (
    <>
      <header className="page-head">
        <div>
          <h1>{t('nav.sessions')}</h1>
          <p>{t('nav.sessionsHelp')}</p>
        </div>
        <button
          type="button"
          className="button"
          disabled={busy || view.sessions.length === 0}
          onClick={() => void onClear()}
        >
          <Eraser size={16} aria-hidden="true" />
          {t('sessions.clearRecords')}
        </button>
      </header>

      {notice?.target === 'page' && (
        <NoticeBanner notice={notice} onDismiss={onDismissNotice} className="page-notice" />
      )}

      {view.sessions.length === 0 ? (
        <div className="empty panel">
          <span className="empty-icon">
            <Activity size={30} aria-hidden="true" />
          </span>
          <h2>{t('sessions.diagnosticsEmpty')}</h2>
          <p>{t('sessions.diagnosticsEmptyHelp')}</p>
        </div>
      ) : (
        <ul className="session-list">
          {view.sessions.map((session) => (
            <SessionRow
              key={session.id}
              session={session}
              expanded={expanded.has(session.id)}
              log={logs[session.id]}
              requested={requested[session.id] ?? null}
              onToggle={() => toggle(session.id)}
              onOpenLog={() => onOpenLog(session.id)}
              onOpenSasuser={() => onOpenSasuser(session.id)}
              onShowLog={(severity) => void showLog(session.id, severity)}
            />
          ))}
        </ul>
      )}
    </>
  )
}
