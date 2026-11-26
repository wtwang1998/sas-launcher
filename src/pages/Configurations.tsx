import { Layers3 } from 'lucide-react'
import { useId, useState } from 'react'
import type { Config, ConfigBase, ConfigRef, View } from '../api'
import { ConfigRow } from '../components/ConfigRow'
import { NoticeBanner } from '../components/Notice'
import {
  configLabel,
  factoryPresets,
  groupConfigs,
  localeLanguageName,
  matchPreset,
  optionsWarning,
  usageCount,
} from '../lib/config'
import type { Notice } from '../lib/notice'
import { useI18n } from '../i18n/provider'

export type ConfigurationsProps = {
  view: View
  busy: boolean
  notice: Notice | null
  onDismissNotice: () => void
  /** Set while a half-written project is parked: renders the return affordance. */
  returnDraft: (() => void) | null
  onCreate: (base: ConfigBase, name: string | null) => Promise<void>
  onUpdate: (id: string, encoding: string, locale: string) => Promise<boolean>
  onDelete: (id: string) => Promise<void>
  onStartSetup: (id: string) => Promise<void>
  onFinishSetup: (id: string, save: boolean) => Promise<void>
}

type Editing = { config: Config; encoding: string; locale: string }

export function Configurations({
  view,
  busy,
  notice,
  onDismissNotice,
  returnDraft,
  onCreate,
  onUpdate,
  onDelete,
  onStartSetup,
  onFinishSetup,
}: ConfigurationsProps) {
  const { language, t } = useI18n()
  const { users, builtIns } = groupConfigs(view.configs, language)
  const presets = factoryPresets(view.configs)
  const [editing, setEditing] = useState<Editing | null>(null)
  const [setupPrompt, setSetupPrompt] = useState<Config | null>(null)
  const dialogTitleId = useId()
  const fieldId = useId()

  const setupOf = (id: string) =>
    view.sessions.find((session) => session.kind === 'setup' && session.configId === id) ?? null

  function baseRef(config: Config): ConfigBase {
    return config.reference.kind === 'user'
      ? { kind: 'user', id: config.reference.id }
      : { kind: 'builtIn', code: config.reference.code }
  }

  function idOf(reference: ConfigRef): string {
    return reference.kind === 'user' ? reference.id : reference.code
  }

  async function saveEditing() {
    if (!editing) return
    const { config, encoding, locale } = editing
    // Stay open on failure: the dialog carries the reason next to the button.
    const saved = await onUpdate(idOf(config.reference), encoding.trim(), locale.trim())
    if (saved) setEditing(null)
  }

  const changedOptions = editing
    ? editing.encoding.trim() !== (editing.config.encoding ?? '') ||
      editing.locale.trim() !== (editing.config.locale ?? '')
    : false
  const check = editing
    ? optionsWarning(
        editing.encoding,
        editing.locale,
        changedOptions ? 'unverified' : editing.config.preset,
      )
    : null
  const currentPreset = editing ? matchPreset(presets, editing.encoding, editing.locale) : null

  return (
    <>
      <header className="page-head">
        <div>
          <h1>{t('nav.configurations')}</h1>
          <p>{t('nav.configurationsHelp')}</p>
        </div>
        {returnDraft && (
          <button type="button" className="button" onClick={returnDraft}>
            {t('project.returnToDraft')}
          </button>
        )}
      </header>

      {notice?.target === 'page' && (
        <NoticeBanner notice={notice} onDismiss={onDismissNotice} className="page-notice" />
      )}

      <section className="config-group" aria-labelledby={`${fieldId}-users`}>
        <h2 id={`${fieldId}-users`}>{t('configurations.userConfigurations')}</h2>
        {users.length === 0 ? (
          <div className="empty panel config-empty">
            <span className="empty-icon">
              <Layers3 size={28} strokeWidth={1.5} aria-hidden="true" />
            </span>
            <h3>{t('configurations.noUserConfigurations')}</h3>
            <p>{t('configurations.noUserConfigurationsHelp')}</p>
          </div>
        ) : (
          <ul className="config-list">
            {users.map((config) => (
              <ConfigRow
                key={idOf(config.reference)}
                config={config}
                busy={busy}
                usage={usageCount(view.projects, config.reference)}
                setupSession={setupOf(idOf(config.reference))}
                onDerive={() => void onCreate(baseRef(config), null)}
                onEdit={() =>
                  setEditing({
                    config,
                    encoding: config.encoding ?? '',
                    locale: config.locale ?? '',
                  })
                }
                onOpenSetup={() => setSetupPrompt(config)}
                onDelete={() => void onDelete(idOf(config.reference))}
                onFinishSetup={(save) => void onFinishSetup(idOf(config.reference), save)}
              />
            ))}
          </ul>
        )}
      </section>

      <section className="config-group" aria-labelledby={`${fieldId}-builtins`}>
        <h2 id={`${fieldId}-builtins`}>{t('configurations.builtInConfigurations')}</h2>
        {builtIns.length === 0 ? (
          <p className="muted config-empty-note">{t('configurations.noBuiltInConfigurations')}</p>
        ) : (
          <ul className="config-list tiles">
            {builtIns.map((config) => (
              <ConfigRow
                key={idOf(config.reference)}
                config={config}
                busy={busy}
                layout="tile"
                usage={usageCount(view.projects, config.reference)}
                setupSession={null}
                onDerive={() => void onCreate(baseRef(config), null)}
                onEdit={() => undefined}
                onOpenSetup={() => undefined}
                onDelete={() => undefined}
                onFinishSetup={() => undefined}
              />
            ))}
          </ul>
        )}
      </section>

      {editing && (
        <div
          className="modal-backdrop"
          onMouseDown={(event) => {
            if (event.target === event.currentTarget) setEditing(null)
          }}
        >
          <div
            className="modal panel"
            role="dialog"
            aria-modal="true"
            aria-labelledby={dialogTitleId}
          >
            <h2 id={dialogTitleId}>
              {t('configurations.editSettingsTitle', { name: configLabel(t, editing.config) })}
            </h2>
            <div className="field">
              <label htmlFor={`${fieldId}-preset`}>{t('config.presetCombination')}</label>
              <select
                id={`${fieldId}-preset`}
                value={currentPreset?.key ?? ''}
                disabled={busy}
                onChange={(event) => {
                  const preset = presets.find((item) => item.key === event.target.value)
                  if (preset)
                    setEditing((current) =>
                      current
                        ? { ...current, encoding: preset.encoding, locale: preset.locale }
                        : current,
                    )
                }}
              >
                {!currentPreset && <option value="">{t('config.presetCustom')}</option>}
                {presets.map((preset) => (
                  <option key={preset.key} value={preset.key}>
                    {(localeLanguageName(language, preset.locale) ?? preset.locale) +
                      ' · ' +
                      preset.encoding.toUpperCase()}
                  </option>
                ))}
              </select>
            </div>
            <div className="field">
              <label htmlFor={`${fieldId}-encoding`}>{t('config.encoding')}</label>
              <input
                id={`${fieldId}-encoding`}
                value={editing.encoding}
                disabled={busy}
                placeholder={t('config.encodingHint')}
                onChange={(event) => setEditing({ ...editing, encoding: event.target.value })}
              />
            </div>
            <div className="field">
              <label htmlFor={`${fieldId}-locale`}>{t('config.locale')}</label>
              <input
                id={`${fieldId}-locale`}
                value={editing.locale}
                disabled={busy}
                placeholder={t('config.localeHint')}
                onChange={(event) => setEditing({ ...editing, locale: event.target.value })}
              />
            </div>
            {check?.warning && <p className="option-warning">{t(check.warning)}</p>}
            <p className="field-hint">{t('config.editorSettingsNotIncluded')}</p>
            {notice?.target === 'dialog' && (
              <NoticeBanner notice={notice} onDismiss={onDismissNotice} className="form-notice" />
            )}
            <div className="form-actions">
              <button
                type="button"
                className="button"
                disabled={busy}
                onClick={() => setEditing(null)}
              >
                {t('common.cancel')}
              </button>
              <button
                type="button"
                className="button primary"
                disabled={busy}
                onClick={() => void saveEditing()}
              >
                {t('common.save')}
              </button>
            </div>
          </div>
        </div>
      )}

      {setupPrompt && (
        <div
          className="modal-backdrop"
          onMouseDown={(event) => {
            if (event.target === event.currentTarget) setSetupPrompt(null)
          }}
        >
          <div
            className="modal panel"
            role="dialog"
            aria-modal="true"
            aria-labelledby={`${dialogTitleId}-setup`}
          >
            <h2 id={`${dialogTitleId}-setup`}>
              {t('configurations.setupDialogTitle', { name: configLabel(t, setupPrompt) })}
            </h2>
            <p className="modal-body">{t('configurations.setupDialogBody')}</p>
            <p className="field-hint">{t('config.editorSettingsNotIncluded')}</p>
            <div className="form-actions">
              <button
                type="button"
                className="button"
                disabled={busy}
                onClick={() => setSetupPrompt(null)}
              >
                {t('common.cancel')}
              </button>
              <button
                type="button"
                className="button primary"
                disabled={busy}
                onClick={() => {
                  const id = idOf(setupPrompt.reference)
                  setSetupPrompt(null)
                  void onStartSetup(id)
                }}
              >
                {t('configurations.setupDialogConfirm')}
              </button>
            </div>
          </div>
        </div>
      )}
    </>
  )
}
