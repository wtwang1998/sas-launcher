import { useEffect, useId, useState } from 'react'
import type { Preference, ThemePreference, View } from '../api'
import { NoticeBanner } from '../components/Notice'
import { Switch } from '../components/Switch'
import { localeLanguageName } from '../lib/config'
import type { Notice } from '../lib/notice'
import { startMinimizedActive } from '../lib/settings'
import { useI18n } from '../i18n/provider'

export type SettingsProps = {
  view: View
  busy: boolean
  notice: Notice | null
  onDismissNotice: () => void
  onTheme: (preference: ThemePreference) => void
  onLanguage: (preference: Preference) => void
  onSavePath: (path: string) => Promise<void>
  onBrowse: () => void
  onAutoDetect: () => Promise<void>
  onStartup: (enabled: boolean) => void
  onStartMinimized: (enabled: boolean) => void
  onCloseToTray: (enabled: boolean) => void
}

export function Settings({
  view,
  busy,
  notice,
  onDismissNotice,
  onTheme,
  onLanguage,
  onSavePath,
  onBrowse,
  onAutoDetect,
  onStartup,
  onStartMinimized,
  onCloseToTray,
}: SettingsProps) {
  const { language, t } = useI18n()
  const id = useId()
  const [path, setPath] = useState(view.exe ?? '')
  const [dirty, setDirty] = useState(false)

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
          <h1>{t('nav.settings')}</h1>
          <p>{t('nav.settingsHelp')}</p>
        </div>
      </header>

      <section className="settings-group panel" aria-labelledby={`${id}-interface`}>
        <h2 id={`${id}-interface`}>{t('settings.interfaceGroup')}</h2>
        <div className="setting-row">
          <span className="setting-label">{t('settings.theme')}</span>
          <div className="setting-control">
            <select
              aria-label={t('settings.theme')}
              value={view.themePreference}
              disabled={busy}
              onChange={(event) => onTheme(event.target.value as ThemePreference)}
            >
              <option value="system">{t('settings.system')}</option>
              <option value="dark">{t('settings.themeDark')}</option>
              <option value="light">{t('settings.themeLight')}</option>
            </select>
          </div>
        </div>
        <div className="setting-row">
          <span className="setting-label">{t('settings.language')}</span>
          <div className="setting-control">
            <select
              aria-label={t('settings.language')}
              value={view.languagePreference}
              disabled={busy}
              onChange={(event) => onLanguage(event.target.value as Preference)}
            >
              <option value="system">{t('settings.system')}</option>
              <option value="zh-Hans">{t('settings.languageZhHans')}</option>
              <option value="zh-Hant">{t('settings.languageZhHant')}</option>
              <option value="en">{t('settings.languageEnglish')}</option>
            </select>
          </div>
        </div>
        {view.languagePreference === 'system' && (
          <p className="setting-hint">
            {t('settings.systemResolved', {
              language: localeLanguageName(language, view.systemLanguage) ?? view.systemLanguage,
            })}
          </p>
        )}
        {notice?.target === 'interface' && (
          <NoticeBanner notice={notice} onDismiss={onDismissNotice} className="section-notice" />
        )}
      </section>

      <section className="settings-group panel" aria-labelledby={`${id}-sas`}>
        <h2 id={`${id}-sas`}>{t('settings.sasGroup')}</h2>
        <div className="setting-row wide">
          <span className="setting-label">{t('settings.sasPath')}</span>
          <div className="setting-control path-control">
            <input
              aria-label={t('settings.sasPath')}
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
              {t('common.browse')}
            </button>
            <button
              type="button"
              className="button small primary"
              disabled={busy || !dirty || !trimmed}
              onClick={() => void save()}
            >
              {t('settings.savePath')}
            </button>
            <button
              type="button"
              className="button small"
              disabled={busy}
              onClick={() => void onAutoDetect()}
            >
              {t('common.autoDetect')}
            </button>
          </div>
        </div>
        <p className="setting-hint">{t('settings.sasPathHint')}</p>
        {notice?.target === 'sas' && (
          <NoticeBanner notice={notice} onDismiss={onDismissNotice} className="section-notice" />
        )}
      </section>

      <section className="settings-group panel" aria-labelledby={`${id}-window`}>
        <h2 id={`${id}-window`}>{t('settings.windowGroup')}</h2>
        <div className="setting-row">
          <span className="setting-label">
            <span id={`${id}-startup`}>{t('settings.startup')}</span>
            <span className="setting-state">{t('settings.startupHint')}</span>
          </span>
          <div className="setting-control">
            <Switch
              labelledBy={`${id}-startup`}
              checked={view.autostart}
              disabled={busy}
              onChange={onStartup}
            />
          </div>
        </div>
        {view.autostart && (
          <div className="setting-row">
            <span className="setting-label">
              <span id={`${id}-start-minimized`}>{t('settings.startMinimized')}</span>
              <span className="setting-state">
                {startMinimizedActive(view.autostart, view.startMinimized)
                  ? t('settings.startMinimizedActive')
                  : t('settings.startMinimizedShown')}
              </span>
            </span>
            <div className="setting-control">
              <Switch
                labelledBy={`${id}-start-minimized`}
                checked={view.startMinimized}
                disabled={busy}
                onChange={onStartMinimized}
              />
            </div>
          </div>
        )}
        <div className="setting-row">
          <span className="setting-label">
            <span id={`${id}-close-to-tray`}>{t('settings.closeToTray')}</span>
            <span className="setting-state">{t('settings.closeToTrayHint')}</span>
          </span>
          <div className="setting-control">
            <Switch
              labelledBy={`${id}-close-to-tray`}
              checked={view.closeToTray}
              disabled={busy}
              onChange={onCloseToTray}
            />
          </div>
        </div>
        {notice?.target === 'window' && (
          <NoticeBanner notice={notice} onDismiss={onDismissNotice} className="section-notice" />
        )}
      </section>
    </>
  )
}
