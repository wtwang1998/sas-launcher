import { ChevronRight } from 'lucide-react'
import { useId } from 'react'
import type { Language, Session, SessionStage, SessionStatus } from '../api'
import type { MessageKey, Translator } from '../i18n'
import { issueText } from '../i18n'
import { chainLines } from '../lib/config'
import { formatArgv, formatStartedAt } from '../lib/format'

const statusKeys: Record<SessionStatus, MessageKey> = {
  running: 'session.statusRunning',
  exited: 'session.statusExited',
  setup: 'session.statusSetup',
  cleanupPending: 'session.statusCleanupPending',
  unknown: 'session.statusUnknown',
}

const stageKeys: Record<SessionStage, MessageKey> = {
  exe: 'session.stageExe',
  cfg: 'session.stageCfg',
  profile: 'session.stageProfile',
  spawn: 'session.stageSpawn',
  started: 'session.stageStarted',
}

export function SessionRow({
  session,
  language,
  t,
  expanded,
  onToggle,
  onOpenLog,
  onOpenSasuser,
}: {
  session: Session
  language: Language
  t: Translator
  expanded: boolean
  onToggle: () => void
  onOpenLog: () => void
  onOpenSasuser: () => void
}) {
  const detailId = useId()
  const stage = t(stageKeys[session.stage])
  const failed = session.status === 'exited' && session.stage !== 'started'
  return (
    <li className="session-row panel">
      <div className="session-summary">
        <button
          type="button"
          className="session-toggle"
          aria-expanded={expanded}
          aria-controls={expanded ? detailId : undefined}
          aria-label={t('session.expandSession', { name: session.name })}
          onClick={onToggle}
        >
          <ChevronRight
            size={15}
            className={`disclosure${expanded ? ' expanded' : ''}`}
            aria-hidden="true"
          />
          <span className="session-name">{session.name}</span>
          <span className={`session-status ${session.status}`}>
            {t(statusKeys[session.status])}
          </span>
          <span className="session-stage">
            {failed ? t('session.failedStageWith', { stage }) : t('session.stageWith', { stage })}
          </span>
          {session.exitCode !== null && (
            <span className="session-exit">
              {t('session.exitCodeValue', { code: session.exitCode })}
            </span>
          )}
        </button>
        <div className="session-actions">
          <button type="button" className="button small" onClick={onOpenLog}>
            {t('sessions.openLog')}
          </button>
          {session.status === 'running' && (
            <button type="button" className="button small" onClick={onOpenSasuser}>
              {t('sessions.openSasuser')}
            </button>
          )}
        </div>
      </div>
      {expanded && (
        <div className="session-detail" id={detailId}>
          <h3>
            {t('session.chainHeading')}
            {session.chain.label ? ` · ${session.chain.label}` : ''}
          </h3>
          <dl className="chain-list">
            {chainLines(session.chain, language, t).map((line) => (
              <div className="chain-row" key={line.key}>
                <dt>{t(line.key)}</dt>
                <dd>{line.value ?? t('session.chainNone')}</dd>
              </div>
            ))}
          </dl>
          <h3>{t('session.argvHeading')}</h3>
          {session.argv.length > 0 ? (
            <code className="argv">{formatArgv(session.argv)}</code>
          ) : (
            <p className="muted">{t('session.noArgv')}</p>
          )}
          {session.failure && (
            <p className="session-failure" role="status">
              {issueText(language, session.failure)}
            </p>
          )}
          <p className="session-meta">
            {t('session.startedAt', { time: formatStartedAt(session.startedAt, language) })}
            {' · '}
            {t('session.logCounts', { errors: session.errors, warnings: session.warnings })}
            {' · '}
            {session.sasuserWritable ? t('session.sasuserWritable') : t('session.sasuserReadOnly')}
            {' · '}
            {t(
              session.kind === 'setup' ? 'session.sessionKindSetup' : 'session.sessionKindProject',
            )}
          </p>
          {session.hints.length > 0 && (
            <>
              <h3>{t('session.hintsHeading')}</h3>
              <ul className="session-hints">
                {session.hints.map((hint) => (
                  <li key={hint.code}>{issueText(language, hint)}</li>
                ))}
              </ul>
            </>
          )}
        </div>
      )}
    </li>
  )
}
