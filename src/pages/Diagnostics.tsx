import { Activity, Eraser } from 'lucide-react'
import { useState } from 'react'
import type { View } from '../api'
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
}

export function Diagnostics({
  view,
  busy,
  notice,
  onDismissNotice,
  onClear,
  onOpenLog,
  onOpenSasuser,
}: DiagnosticsProps) {
  const t = useT()
  const [expanded, setExpanded] = useState<ReadonlySet<string>>(() => new Set())

  function toggle(id: string) {
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
              onToggle={() => toggle(session.id)}
              onOpenLog={() => onOpenLog(session.id)}
              onOpenSasuser={() => onOpenSasuser(session.id)}
            />
          ))}
        </ul>
      )}
    </>
  )
}
