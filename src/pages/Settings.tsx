import { useEffect, useId, useState } from 'react'
import type { Preference, View } from '../api'
import type { Translator } from '../i18n'

export type SettingsProps = {
  view: View
  t: Translator
  busy: boolean
  onLanguage: (preference: Preference) => void
  onSavePath: (path: string) => Promise<void>
  onBrowse: () => void
  onAutoDetect: () => Promise<void>
}

export function Settings({
  view,
  t,
  busy,
  onLanguage,
  onSavePath,
  onBrowse,
  onAutoDetect,
}: SettingsProps) {
  const id = useId()
  const [path, setPath] = useState(view.exe ?? '')
  const [dirty, setDirty] = useState(false)
  const found = view.exe !== null

  useEffect(() => {
    if (!dirty) setPath(view.exe ?? '')
  }, [view.exe, dirty])

  const trimmed = path.trim()

  async function save() {
    if (!trimmed) return
    setDirty(false)
    await onSavePath(trimmed)
  }

  return (
    <>
      <header className="page-head">
        <div>
          <h1>{t('settings')}</h1>
          <p>{t('settingsHelp')}</p>
        </div>
      </header>

      <section className="settings-group panel" aria-labelledby={`${id}-interface`}>
        <h2 id={`${id}-interface`}>{t('interfaceGroup')}</h2>
        <div className="setting-row">
          <span className="setting-label">{t('language')}</span>
          <div className="setting-control">
            <select
              aria-label={t('language')}
              value={view.languagePreference}
              disabled={busy}
              onChange={(event) => onLanguage(event.target.value as Preference)}
            >
              <option value="system">{t('system')}</option>
              <option value="zh-Hans">{t('languageZhHans')}</option>
              <option value="en">{t('languageEnglish')}</option>
            </select>
          </div>
        </div>
      </section>

      <section className="settings-group panel" aria-labelledby={`${id}-sas`}>
        <h2 id={`${id}-sas`}>{t('sasGroup')}</h2>
        <div className="setting-row">
          <span className="setting-label">
            {t('sasPath')}
            <span className={`setting-state ${found ? 'found' : ''}`}>
              <span className={`status-dot ${found ? 'green' : ''}`} aria-hidden="true" />
              {found ? t('sasDetected') : t('noInstallation')}
            </span>
          </span>
          <div className="setting-control path-control">
            <input
              aria-label={t('sasPath')}
              value={path}
              disabled={busy}
              spellCheck={false}
              placeholder={'C:\\Program Files\\SASHome\\SASFoundation\\9.4\\sas.exe'}
              onChange={(event) => {
                setDirty(true)
                setPath(event.target.value)
              }}
              onKeyDown={(event) => {
                if (event.key === 'Enter' && dirty) {
                  event.preventDefault()
                  void save()
                }
              }}
            />
            <button type="button" className="button small" disabled={busy} onClick={onBrowse}>
              {t('browse')}
            </button>
            <button
              type="button"
              className="button small primary"
              disabled={busy || !dirty || !trimmed}
              onClick={() => void save()}
            >
              {t('savePath')}
            </button>
            <button
              type="button"
              className="button small"
              disabled={busy}
              onClick={() => void onAutoDetect()}
            >
              {t('autoDetect')}
            </button>
          </div>
        </div>
        <p className="setting-hint">{t('sasPathHint')}</p>
      </section>
    </>
  )
}
