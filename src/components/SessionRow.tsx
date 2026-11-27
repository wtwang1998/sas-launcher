import { Check, ChevronRight, CircleAlert, Copy } from 'lucide-react'
import { useEffect, useId, useRef, useState } from 'react'
import type {
  LogEntry,
  LogSeverity,
  Session,
  SessionLog,
  SessionStage,
  SessionStatus,
} from '../api'
import { LOG_ENTRY_CAP } from '../api'
import type { MessageKey } from '../i18n'
import { issueText } from '../i18n'
import { chainLines } from '../lib/config'
import { formatArgv, formatStartedAt } from '../lib/format'
import { useI18n } from '../i18n/provider'

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

/** 复制反馈在被点的按钮上停留这么久，然后退回常态。 */
const COPY_FEEDBACK_MS = 2000

export function SessionRow({
  session,
  expanded,
  log,
  requested,
  onToggle,
  onOpenLog,
  onOpenSasuser,
  onShowLog,
}: {
  session: Session
  expanded: boolean
  log: SessionLog | 'loading' | undefined
  requested: LogSeverity | null
  onToggle: () => void
  onOpenLog: () => void
  onOpenSasuser: () => void
  onShowLog: (severity: LogSeverity) => void
}) {
  const { language, t } = useI18n()
  const detailId = useId()
  const listRef = useRef<HTMLOListElement>(null)
  const focusedRef = useRef<string | null>(null)
  const chipsRef = useRef<HTMLDivElement>(null)
  const copyTimer = useRef<number | null>(null)
  const [copied, setCopied] = useState<{ line: number; ok: boolean } | null>(null)
  const copiedLine = copied?.line ?? null
  const copiedOk = copied?.ok === true
  const closedRef = useRef<LogSeverity | null>(null)
  const stage = t(stageKeys[session.stage])
  const failed = session.status === 'exited' && session.stage !== 'started'
  const report = log === 'loading' ? null : (log ?? null)
  const hasFindings = session.errors + session.warnings > 0
  // Once the panel has its own report, the chips quote that scan: the list
  // below them comes from the same read, so both numbers must agree.
  const counted = report?.state === 'ok' ? report : session
  const chipCounts = [counted.errors, counted.warnings]

  // Focusing the first row is how "jump to this severity" is observed: the
  // browser scrolls a focused element into view by itself.
  useEffect(() => {
    if (report?.state !== 'ok' || !requested) return
    const word = `${requested}:${report.entries.find((entry) => entry.severity === requested)?.line}`
    if (focusedRef.current === word) return
    const row = listRef.current?.querySelector<HTMLElement>(`[data-severity="${requested}"]`)
    if (!row) return
    focusedRef.current = word
    row.focus()
  }, [report, requested])

  // Closing the list unmounts the focused row, so the caret goes back to the
  // chip that was pressed: the same place the user was standing on. The copy
  // feedback belongs to a row of that list, so it leaves with it too.
  useEffect(() => {
    if (copyTimer.current !== null) {
      window.clearTimeout(copyTimer.current)
      copyTimer.current = null
    }
    setCopied(null)

    if (requested) {
      closedRef.current = requested
      return
    }
    const previous = closedRef.current
    if (!previous) return
    closedRef.current = null
    chipsRef.current?.querySelector<HTMLElement>(`.log-chip.${previous}`)?.focus()
  }, [requested])

  useEffect(
    () => () => {
      if (copyTimer.current !== null) window.clearTimeout(copyTimer.current)
    },
    [],
  )

  /** 复制结果只落在被点的那一行：名字与图标一起变，2 秒后一起退回。 */
  function copyLabel(line: number) {
    if (copiedLine !== line) return t('session.logCopyLine', { line })
    return copiedOk
      ? t('session.logCopiedLine', { line })
      : t('session.logCopyFailedLine', { line })
  }

  function copyClass(line: number) {
    if (copiedLine !== line) return 'button small session-log-copy'
    return `button small session-log-copy ${copiedOk ? 'copied' : 'failed'}`
  }

  function copyIcon(line: number) {
    if (copiedLine !== line) return <Copy size={16} aria-hidden="true" />
    return copiedOk ? (
      <Check size={16} aria-hidden="true" />
    ) : (
      <CircleAlert size={16} aria-hidden="true" />
    )
  }

  function flashCopy(line: number, ok: boolean) {
    if (copyTimer.current !== null) window.clearTimeout(copyTimer.current)
    setCopied({ line, ok })
    copyTimer.current = window.setTimeout(() => setCopied(null), COPY_FEEDBACK_MS)
  }

  async function copyEntry(entry: LogEntry) {
    try {
      await navigator.clipboard.writeText(`sas.log:${entry.line} ${entry.text}`)
      flashCopy(entry.line, true)
    } catch {
      flashCopy(entry.line, false)
    }
  }

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
          {hasFindings && (
            <span className="session-hint">
              {t('session.logHint', { errors: session.errors, warnings: session.warnings })}
            </span>
          )}
        </button>
        <div className="session-actions">
          <button type="button" className="button small" onClick={onOpenLog}>
            {t('sessions.openFullLog')}
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
            {session.sasuserWritable ? t('session.sasuserWritable') : t('session.sasuserReadOnly')}
            {' · '}
            {t(
              session.kind === 'setup' ? 'session.sessionKindSetup' : 'session.sessionKindProject',
            )}
          </p>
          {session.chain.log === null ? (
            <p className="session-log-empty muted">{t('session.logNoLog')}</p>
          ) : (
            chipCounts[0] + chipCounts[1] > 0 && (
              <div className="session-log-counts" ref={chipsRef}>
                {chipCounts[0] > 0 && (
                  <button
                    type="button"
                    className="log-chip error"
                    aria-pressed={requested === 'error'}
                    onClick={() => onShowLog('error')}
                  >
                    {t('session.logErrors', { count: chipCounts[0] })}
                  </button>
                )}
                {chipCounts[1] > 0 && (
                  <button
                    type="button"
                    className="log-chip warning"
                    aria-pressed={requested === 'warning'}
                    onClick={() => onShowLog('warning')}
                  >
                    {t('session.logWarnings', { count: chipCounts[1] })}
                  </button>
                )}
              </div>
            )
          )}
          {requested && report && (
            <section className="session-log">
              {report.state === 'unreadable' ? (
                <p className="session-log-empty muted">{t('session.logUnreadable')}</p>
              ) : (
                <>
                  {report.truncated && (
                    <p className="session-log-note muted">
                      {t('session.logTruncated', { count: LOG_ENTRY_CAP })}
                    </p>
                  )}
                  <ol className="session-log-entries" ref={listRef}>
                    {report.entries.map((entry) => (
                      <li
                        key={`${entry.line}-${entry.severity}`}
                        className={`session-log-entry ${entry.severity}`}
                        data-severity={entry.severity}
                        tabIndex={-1}
                      >
                        <span className="session-log-line">{entry.line}</span>
                        <span className="session-log-text">{entry.text}</span>
                        <button
                          type="button"
                          className={copyClass(entry.line)}
                          aria-label={copyLabel(entry.line)}
                          onClick={() => void copyEntry(entry)}
                        >
                          {copyIcon(entry.line)}
                        </button>
                      </li>
                    ))}
                  </ol>
                </>
              )}
            </section>
          )}
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
