import { Check, CircleAlert, X } from 'lucide-react'
import { useEffect, useRef } from 'react'
import type { Language } from '../api'
import { issueText, type Translator } from '../i18n'
import type { Notice } from '../lib/notice'

/**
 * The result of the action the user just took, rendered by the page next to
 * that action. Errors are alerts so assistive tech announces them.
 */
export function NoticeBanner({
  notice,
  language,
  t,
  onDismiss,
  onRetry,
  onMigrateStartup,
  className = '',
}: {
  notice: Notice
  language: Language
  t: Translator
  onDismiss: () => void
  /** Offered when the action can simply be run again, e.g. after closing SAS. */
  onRetry?: () => void
  /** Offered when the failure has a repair the launcher can run itself. */
  onMigrateStartup?: (projectId: string) => void
  className?: string
}) {
  const failed = notice.tone === 'error'
  const migrate = notice.tone === 'error' ? notice.action : undefined
  const ref = useRef<HTMLDivElement>(null)
  useEffect(() => {
    // A long form can leave the message below the fold; every new result
    // scrolls itself into view.
    if (!notice) return
    ref.current?.scrollIntoView?.({ block: 'nearest' })
  }, [notice])

  return (
    <div
      ref={ref}
      className={`banner ${failed ? 'error' : 'success'} notice-block ${className}`.trim()}
      role={failed ? 'alert' : 'status'}
    >
      {failed ? (
        <CircleAlert size={18} aria-hidden="true" />
      ) : (
        <Check size={18} aria-hidden="true" />
      )}
      <span>{failed ? issueText(language, notice.error) : t(notice.key, notice.values)}</span>
      {failed && onRetry && (
        <button type="button" className="button small" onClick={onRetry}>
          {t('retry')}
        </button>
      )}
      {onMigrateStartup && migrate?.kind === 'migrateStartup' && (
        <button
          type="button"
          className="button small"
          onClick={() => onMigrateStartup(migrate.projectId)}
        >
          {t('migrateStartup')}
        </button>
      )}
      <button
        type="button"
        className="icon-button"
        aria-label={t(failed ? 'closeError' : 'closeMessage')}
        onClick={onDismiss}
      >
        <X size={16} />
      </button>
    </div>
  )
}
