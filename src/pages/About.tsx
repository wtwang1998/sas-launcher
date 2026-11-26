import { useId, useState } from 'react'
import type { Language, View } from '../api'
import { issueText, type Translator } from '../i18n'
import { copyVersion, LICENSE } from '../lib/about'
import { Check, Copy, ExternalLink, GitBranch, MessageSquare, SquareTerminal } from 'lucide-react'

export type AboutProps = {
  view: View
  t: Translator
  language: Language
  onOpenRepository: () => Promise<void>
  onOpenIssues: () => Promise<void>
}

export function About({ view, t, language, onOpenRepository, onOpenIssues }: AboutProps) {
  const id = useId()
  const [copied, setCopied] = useState(false)
  /** A link that would not open says so where it was clicked, not in a banner. */
  const [failure, setFailure] = useState<unknown>(null)

  async function copy() {
    setFailure(null)
    try {
      await navigator.clipboard.writeText(copyVersion(view.version, view.commit))
      setCopied(true)
      window.setTimeout(() => setCopied(false), 2000)
    } catch (error) {
      setFailure(error)
    }
  }

  async function open(action: () => Promise<void>) {
    setFailure(null)
    try {
      await action()
    } catch (error) {
      setFailure(error)
    }
  }

  return (
    <>
      <header className="page-head">
        <div>
          <h1>{t('about')}</h1>
          <p>{t('aboutHelp')}</p>
        </div>
      </header>

      <section className="settings-group panel" aria-labelledby={`${id}-version`}>
        <h2 id={`${id}-version`}>{t('versionGroup')}</h2>
        <div className="about-identity">
          <span className="card-icon">
            <SquareTerminal size={22} strokeWidth={1.8} aria-hidden="true" />
          </span>
          <span className="about-identity-name">
            {t('title')}
            <span className="chip chip-name about-version">v{view.version}</span>
          </span>
          <button type="button" className="button small about-copy" onClick={() => void copy()}>
            {copied ? (
              <Check size={15} aria-hidden="true" />
            ) : (
              <Copy size={15} aria-hidden="true" />
            )}
            {copied ? t('copied') : t('copyVersion')}
          </button>
        </div>
        <dl className="diagnostic about-facts">
          <dt>{t('buildCommit')}</dt>
          <dd className="config-code">{view.commit}</dd>
        </dl>
      </section>

      <section className="settings-group panel" aria-labelledby={`${id}-links`}>
        <h2 id={`${id}-links`}>{t('linksGroup')}</h2>
        <button type="button" className="about-link" onClick={() => void open(onOpenRepository)}>
          <GitBranch size={16} aria-hidden="true" />
          <span>{t('repository')}</span>
          <ExternalLink className="about-link-arrow" size={15} aria-hidden="true" />
        </button>
        <button type="button" className="about-link" onClick={() => void open(onOpenIssues)}>
          <MessageSquare size={16} aria-hidden="true" />
          <span>{t('reportIssue')}</span>
          <ExternalLink className="about-link-arrow" size={15} aria-hidden="true" />
        </button>
        {failure !== null && (
          <p className="about-error" role="status">
            {issueText(language, failure)}
          </p>
        )}
      </section>

      <p className="about-license">{LICENSE}</p>
    </>
  )
}
