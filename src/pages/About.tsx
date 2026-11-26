import {
  Check,
  CircleAlert,
  Copy,
  Download,
  ExternalLink,
  GitBranch,
  MessageSquare,
  RefreshCw,
  SquareTerminal,
} from 'lucide-react'
import { useId, useState } from 'react'
import type { Language, View } from '../api'
import { issueText, type MessageKey, type Translator } from '../i18n'
import { copyVersion, LICENSE } from '../lib/about'
import { checkForUpdate, type UpdateCheck } from '../lib/update'

export type AboutProps = {
  view: View
  t: Translator
  language: Language
  onOpenRepository: () => Promise<void>
  onOpenIssues: () => Promise<void>
  onOpenRelease: (tag: string) => Promise<void>
}

const FAILED: Record<string, MessageKey> = {
  timeout: 'about.updateTimeout',
  offline: 'about.updateOffline',
  rateLimited: 'about.updateRateLimited',
  unreadable: 'about.updateUnreadable',
}

function statusIcon(check: UpdateCheck) {
  if (check.kind === 'latest') {
    return <Check size={16} aria-hidden="true" className="about-status-icon ok" />
  }
  if (check.kind === 'available') {
    return <Download size={16} aria-hidden="true" className="about-status-icon info" />
  }
  if (check.kind === 'failed') {
    return <CircleAlert size={16} aria-hidden="true" className="about-status-icon error" />
  }
  return <RefreshCw size={16} aria-hidden="true" className="about-status-icon" />
}

function statusText(check: UpdateCheck, t: Translator, version: string) {
  switch (check.kind) {
    case 'checking':
      return t('about.updateChecking')
    case 'latest':
      return t('about.updateLatest', { version: `v${version}` })
    case 'available':
      return check.prerelease
        ? t('about.updatePreRelease', { version: check.tag })
        : t('about.updateAvailable', { version: check.tag })
    case 'failed':
      return t(FAILED[check.reason])
    default:
      return ''
  }
}

export function About({
  view,
  t,
  language,
  onOpenRepository,
  onOpenIssues,
  onOpenRelease,
}: AboutProps) {
  const id = useId()
  const [copied, setCopied] = useState(false)
  const [check, setCheck] = useState<UpdateCheck>({ kind: 'idle' })
  /** A link that would not open says so under the control that asked for it. */
  const [failure, setFailure] = useState<{ where: 'links' | 'update'; error: unknown } | null>(null)

  async function copy() {
    setFailure(null)
    try {
      await navigator.clipboard.writeText(copyVersion(view.version, view.commit))
      setCopied(true)
      window.setTimeout(() => setCopied(false), 2000)
    } catch (error) {
      setFailure({ where: 'links', error })
    }
  }

  async function open(where: 'links' | 'update', action: () => Promise<void>) {
    setFailure(null)
    try {
      await action()
    } catch (error) {
      setFailure({ where, error })
    }
  }

  async function runCheck() {
    setFailure(null)
    setCheck({ kind: 'checking' })
    setCheck(await checkForUpdate(view.version))
  }

  return (
    <>
      <header className="page-head">
        <div>
          <h1>{t('nav.about')}</h1>
          <p>{t('nav.aboutHelp')}</p>
        </div>
      </header>

      <section className="settings-group panel" aria-labelledby={`${id}-version`}>
        <h2 id={`${id}-version`}>{t('about.versionGroup')}</h2>
        <div className="about-identity">
          <span className="card-icon">
            <SquareTerminal size={22} strokeWidth={1.8} aria-hidden="true" />
          </span>
          <span className="about-identity-name">
            {t('app.title')}
            <span className="chip chip-name about-version">v{view.version}</span>
          </span>
          <button type="button" className="button small about-copy" onClick={() => void copy()}>
            {copied ? (
              <Check size={15} aria-hidden="true" />
            ) : (
              <Copy size={15} aria-hidden="true" />
            )}
            {copied ? t('notice.copied') : t('about.copyVersion')}
          </button>
        </div>
        <dl className="diagnostic about-facts">
          <dt>{t('about.buildCommit')}</dt>
          <dd className="config-code">{view.commit}</dd>
        </dl>
      </section>

      <section className="settings-group panel" aria-labelledby={`${id}-update`}>
        <h2 id={`${id}-update`}>{t('about.updateGroup')}</h2>
        <div className="setting-row">
          <span className="setting-label">{t('about.checkUpdate')}</span>
          <div className="setting-control">
            <button
              type="button"
              className="button small"
              disabled={check.kind === 'checking'}
              onClick={() => void runCheck()}
            >
              <RefreshCw size={15} aria-hidden="true" />
              {t('about.checkUpdate')}
            </button>
          </div>
        </div>
        {check.kind !== 'idle' && (
          <div className="about-status" role="status">
            {statusIcon(check)}
            <span className="about-status-text">{statusText(check, t, view.version)}</span>
            {check.kind === 'available' && (
              <button
                type="button"
                className="button small primary"
                onClick={() => void open('update', () => onOpenRelease(check.tag))}
              >
                {t('common.openRelease')}
              </button>
            )}
          </div>
        )}
        {failure?.where === 'update' && (
          <p className="about-error" role="status">
            {issueText(language, failure.error)}
          </p>
        )}
      </section>

      <section className="settings-group panel" aria-labelledby={`${id}-links`}>
        <h2 id={`${id}-links`}>{t('about.linksGroup')}</h2>
        <button
          type="button"
          className="about-link"
          onClick={() => void open('links', onOpenRepository)}
        >
          <GitBranch size={16} aria-hidden="true" />
          <span>{t('about.repository')}</span>
          <ExternalLink className="about-link-arrow" size={15} aria-hidden="true" />
        </button>
        <button
          type="button"
          className="about-link"
          onClick={() => void open('links', onOpenIssues)}
        >
          <MessageSquare size={16} aria-hidden="true" />
          <span>{t('about.reportIssue')}</span>
          <ExternalLink className="about-link-arrow" size={15} aria-hidden="true" />
        </button>
        {failure?.where === 'links' && (
          <p className="about-error" role="status">
            {issueText(language, failure.error)}
          </p>
        )}
      </section>

      <p className="about-license">{LICENSE}</p>
    </>
  )
}
