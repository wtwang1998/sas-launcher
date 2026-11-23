import { Activity, Eraser } from 'lucide-react'
import { useState } from 'react'
import type { Language, View } from '../api'
import { SessionRow } from '../components/SessionRow'
import type { Translator } from '../i18n'

export type DiagnosticsProps = {
  view: View
  language: Language
  t: Translator
  busy: boolean
  onClear: () => Promise<void>
  onOpenLog: (id: string) => void
  onOpenSasuser: (id: string) => void
}

export function Diagnostics({
  view,
  language,
  t,
  busy,
  onClear,
  onOpenLog,
  onOpenSasuser,
}: DiagnosticsProps) {
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
          <h1>{t('sessions')}</h1>
          <p>{t('sessionsHelp')}</p>
        </div>
        <button
          type="button"
          className="button"
          disabled={busy || view.sessions.length === 0}
          onClick={() => void onClear()}
        >
          <Eraser size={16} aria-hidden="true" />
          {t('clearRecords')}
        </button>
      </header>

      {view.sessions.length === 0 ? (
        <div className="empty panel">
          <span className="empty-icon">
            <Activity size={30} aria-hidden="true" />
          </span>
          <h2>{t('diagnosticsEmpty')}</h2>
          <p>{t('diagnosticsEmptyHelp')}</p>
        </div>
      ) : (
        <ul className="session-list">
          {view.sessions.map((session) => (
            <SessionRow
              key={session.id}
              session={session}
              language={language}
              t={t}
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
