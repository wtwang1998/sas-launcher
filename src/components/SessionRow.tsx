import { ChevronRight } from 'lucide-react'
import { useId } from 'react'
import type { Language, Session, SessionStage, SessionStatus } from '../api'
import type { MessageKey, Translator } from '../i18n'
import { issueText } from '../i18n'
import { chainLines, formatStartedAt } from '../lib/config'
import { formatArgv } from '../lib/format'

const statusKeys: Record<SessionStatus, MessageKey> = {
  running: 'statusRunning',
  exited: 'statusExited',
  setup: 'statusSetup',
  cleanupPending: 'statusCleanupPending',
  unknown: 'statusUnknown',
}

const stageKeys: Record<SessionStage, MessageKey> = {
  exe: 'stageExe',
  cfg: 'stageCfg',
  profile: 'stageProfile',
  spawn: 'stageSpawn',
  started: 'stageStarted',
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
          aria-label={t('expandSession', { name: session.name })}
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
            {failed ? t('failedStageWith', { stage }) : t('stageWith', { stage })}
          </span>
          {session.exitCode !== null && (
            <span className="session-exit">{t('exitCodeValue', { code: session.exitCode })}</span>
          )}
        </button>
        <div className="session-actions">
          <button type="button" className="button small" onClick={onOpenLog}>
            {t('openLog')}
          </button>
          {session.status === 'running' && (
            <button type="button" className="button small" onClick={onOpenSasuser}>
              {t('openSasuser')}
            </button>
          )}
        </div>
      </div>
      {expanded && (
        <div className="session-detail" id={detailId}>
          <h3>
            {t('chainHeading')}
            {session.chain.label ? ` · ${session.chain.label}` : ''}
          </h3>
          <dl className="chain-list">
            {chainLines(session.chain).map((line) => (
              <div className="chain-row" key={line.key}>
                <dt>{t(line.key)}</dt>
                <dd>{line.value ?? t('chainNone')}</dd>
              </div>
            ))}
          </dl>
          <h3>{t('argvHeading')}</h3>
          {session.argv.length > 0 ? (
            <code className="argv">{formatArgv(session.argv)}</code>
          ) : (
            <p className="muted">{t('noArgv')}</p>
          )}
          {session.failure && (
            <p className="session-failure" role="status">
              {issueText(language, session.failure)}
            </p>
          )}
          <p className="session-meta">
            {t('startedAt', { time: formatStartedAt(session.startedAt, language) })}
            {' · '}
            {t('logCounts', { errors: session.errors, warnings: session.warnings })}
            {' · '}
            {session.sasuserWritable ? t('sasuserWritable') : t('sasuserReadOnly')}
            {' · '}
            {t(session.kind === 'setup' ? 'sessionKindSetup' : 'sessionKindProject')}
          </p>
          {session.hints.length > 0 && (
            <>
              <h3>{t('hintsHeading')}</h3>
              <ul className="session-hints">
                {session.hints.map((hint, index) => (
                  <li key={index}>{issueText(language, hint)}</li>
                ))}
              </ul>
            </>
          )}
        </div>
      )}
    </li>
  )
}
