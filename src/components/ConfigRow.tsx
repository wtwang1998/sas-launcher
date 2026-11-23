import { MoreHorizontal } from 'lucide-react'
import { useEffect, useRef, useState } from 'react'
import type { Config, Language, Session } from '../api'
import { issueText, type Translator } from '../i18n'
import { configLabel, presetLabel } from '../lib/config'
import { Chips } from './Chips'

export type ConfigRowProps = {
  config: Config
  language: Language
  t: Translator
  busy: boolean
  /** How many projects reference this configuration. */
  usage: number
  /** The settings session belonging to this configuration, if any. */
  setupSession: Session | null
  onDerive: () => void
  onEdit: () => void
  onOpenSetup: () => void
  onDelete: () => void
  onFinishSetup: (save: boolean) => void
  /** `tile` packs the row into a compact card for the read-only factory list. */
  layout?: 'row' | 'tile'
}

export function ConfigRow({
  config,
  language,
  t,
  busy,
  usage,
  setupSession,
  onDerive,
  onEdit,
  onOpenSetup,
  onDelete,
  onFinishSetup,
  layout = 'row',
}: ConfigRowProps) {
  const [menuOpen, setMenuOpen] = useState(false)
  const root = useRef<HTMLLIElement>(null)
  const builtIn = config.reference.kind === 'builtIn'
  const setupRunning = setupSession?.status === 'running'
  // The backend reports a settings session that exited and awaits save as
  // `setup`; `exited` is reserved for ordinary launches.
  const setupExited = setupSession?.status === 'setup'

  useEffect(() => {
    if (!menuOpen) return
    const leave = (event: MouseEvent) => {
      if (!root.current?.contains(event.target as Node)) setMenuOpen(false)
    }
    window.addEventListener('mousedown', leave)
    return () => window.removeEventListener('mousedown', leave)
  }, [menuOpen])

  function run(action: () => void) {
    setMenuOpen(false)
    action()
  }

  return (
    <li className={'config-row' + (layout === 'tile' ? ' tile' : '')} ref={root}>
      <div className="config-row-main">
        <div className="config-row-title">
          <span className="config-name">{configLabel(t, config)}</span>
          {!builtIn && config.derivedFrom && (
            <small className="config-derived">
              {t('derivedFrom', { name: config.derivedFrom })}
            </small>
          )}
        </div>
        <div className="config-row-meta">
          {config.error && (
            <span className="config-error">{issueText(language, config.error)}</span>
          )}
          {setupRunning && <span className="config-setup">{t('setupSessionRunning')}</span>}
          {usage > 0 && (
            <span className="config-usage">{t('usedByProjects', { count: usage })}</span>
          )}
          {config.preset !== 'verified' && (
            <span className={'preset ' + config.preset}>{presetLabel(t, config.preset)}</span>
          )}
        </div>
      </div>
      <Chips config={config} language={language} t={t} />
      <div className="config-row-actions">
        {setupExited && (
          <>
            <button
              type="button"
              className="button small"
              disabled={busy}
              onClick={() => onFinishSetup(true)}
            >
              {t('saveSessionSettings')}
            </button>
            <button
              type="button"
              className="button small"
              disabled={busy}
              onClick={() => onFinishSetup(false)}
            >
              {t('discard')}
            </button>
          </>
        )}
        <div className="row-menu">
          <button
            type="button"
            className="icon-button"
            aria-label={t('configActions', { name: configLabel(t, config) })}
            aria-haspopup="menu"
            aria-expanded={menuOpen}
            disabled={busy}
            onClick={() => setMenuOpen((open) => !open)}
          >
            <MoreHorizontal size={16} aria-hidden="true" />
          </button>
          {menuOpen && (
            <div
              className="menu"
              role="menu"
              aria-label={t('configActions', { name: configLabel(t, config) })}
            >
              {!builtIn && (
                <>
                  <button
                    type="button"
                    role="menuitem"
                    className="menu-item"
                    disabled={!!setupRunning}
                    title={setupRunning ? t('setupSessionRunning') : undefined}
                    onClick={() => run(onEdit)}
                  >
                    {t('editSettings')}
                  </button>
                  <button
                    type="button"
                    role="menuitem"
                    className="menu-item"
                    onClick={() => run(onOpenSetup)}
                  >
                    {t('openSasSettings')}
                  </button>
                </>
              )}
              <button
                type="button"
                role="menuitem"
                className="menu-item"
                onClick={() => run(onDerive)}
              >
                {t('deriveAction')}
              </button>
              {!builtIn && (
                <button
                  type="button"
                  role="menuitem"
                  className="menu-item danger"
                  disabled={usage > 0}
                  title={usage > 0 ? t('deleteInUse') : undefined}
                  onClick={() => run(onDelete)}
                >
                  {t('deleteConfiguration')}
                </button>
              )}
            </div>
          )}
        </div>
      </div>
    </li>
  )
}
