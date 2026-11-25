import { type FormEvent, type ReactNode, useId, useState } from 'react'
import type { Config, ConfigRef, Language, SasuserCandidate } from '../api'
import type { Translator } from '../i18n'
import { configReady } from '../lib/config'
import { emptyProfileDraft, type ProfileDraft, profileReady } from '../lib/profile'
import { ConfigChoice } from './ConfigChoice'
import { FieldLabel } from './form-fields'
import { Switch } from './Switch'
import { Tooltip } from './Tooltip'
import { UserFolderChoice } from './UserFolderChoice'

export type ProjectDraft = {
  id: string | null
  name: string
  note: string
  config: ConfigRef | null
  profile: ProfileDraft
  /** The project's own autoexec file; an empty string means "run none". */
  startupFile: string
  /** Whether the session is told where that file is (`SAS_EXECFILEPATH`). */
  startupExposesPath: boolean
}

export const emptyProjectDraft: ProjectDraft = {
  id: null,
  name: '',
  note: '',
  config: null,
  profile: emptyProfileDraft,
  startupFile: '',
  startupExposesPath: false,
}

/**
 * One form for new projects, project edits, and the collapsed temporary launch.
 * The submitted draft already has its name and note trimmed.
 */
export function ProjectForm({
  draft,
  configs,
  defaultConfig,
  profileCandidates,
  language,
  t,
  busy,
  submitLabel,
  onChange,
  onSubmit,
  onCancel,
  onCreateConfiguration,
  onBrowseProfile,
  onBrowseStartupFile,
  startupNeedsMigration = false,
  onMigrateStartup,
  notice,
}: {
  draft: ProjectDraft
  configs: Config[]
  defaultConfig: Config | null
  profileCandidates: SasuserCandidate[]
  language: Language
  t: Translator
  busy: boolean
  submitLabel: string
  onChange: (draft: ProjectDraft) => void
  onSubmit: (draft: ProjectDraft) => void
  onCancel?: () => void
  onCreateConfiguration?: () => void
  onBrowseProfile?: () => void
  onBrowseStartupFile?: () => void
  /** The stored script still reads the variable the session cannot fill. */
  startupNeedsMigration?: boolean
  onMigrateStartup?: () => void
  /** The last save's outcome, shown next to the buttons it belongs to. */
  notice?: ReactNode
}) {
  const nameId = useId()
  const noteId = useId()
  const startupId = useId()
  const exposeId = useId()
  const errorId = useId()
  const [nameMissing, setNameMissing] = useState(false)

  function submit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault()
    const name = draft.name.trim()
    if (!name) {
      setNameMissing(true)
      return
    }
    setNameMissing(false)
    onSubmit({ ...draft, name, note: draft.note.trim() })
  }

  return (
    <form className="editor" noValidate onSubmit={submit}>
      <div className="field">
        <label htmlFor={nameId}>
          <FieldLabel label={t('name')} marker={t('requiredField')} />
        </label>
        <input
          id={nameId}
          name="name"
          value={draft.name}
          maxLength={100}
          placeholder={t('example')}
          disabled={busy}
          aria-invalid={nameMissing || undefined}
          aria-describedby={nameMissing ? errorId : undefined}
          onChange={(event) => {
            setNameMissing(false)
            onChange({ ...draft, name: event.target.value })
          }}
        />
        {nameMissing && (
          <p className="field-error" id={errorId}>
            {t('nameRequired')}
          </p>
        )}
      </div>
      <div className="field">
        <span className="field-title">{t('configSelection')}</span>
        <ConfigChoice
          value={draft.config}
          configs={configs}
          defaultConfig={defaultConfig}
          language={language}
          t={t}
          busy={busy}
          onCreateConfiguration={onCreateConfiguration}
          onChange={(config) => onChange({ ...draft, config })}
        />
      </div>
      <div className="field">
        <span className="field-title">{t('userFolderGroup')}</span>
        <UserFolderChoice
          value={draft.profile}
          candidates={profileCandidates}
          configs={configs}
          language={language}
          t={t}
          busy={busy}
          onBrowse={onBrowseProfile}
          onChange={(profile) => onChange({ ...draft, profile })}
        />
      </div>
      <div className="field">
        <label htmlFor={startupId}>
          <FieldLabel label={t('startupFile')} marker={t('optionalField')} />
        </label>
        <div className="field-path">
          <input
            id={startupId}
            name="startupFile"
            value={draft.startupFile}
            placeholder={t('startupFileHint')}
            spellCheck={false}
            disabled={busy}
            onChange={(event) => onChange({ ...draft, startupFile: event.target.value })}
          />
          {onBrowseStartupFile && (
            <button
              type="button"
              className="button small"
              disabled={busy}
              onClick={onBrowseStartupFile}
            >
              {t('browseStartupFile')}
            </button>
          )}
          {draft.startupFile !== '' && (
            <button
              type="button"
              className="button small"
              disabled={busy}
              onClick={() => onChange({ ...draft, startupFile: '' })}
            >
              {t('clearStartupFile')}
            </button>
          )}
        </div>
        <div className="field-switch">
          <span id={exposeId} className="field-switch-label">
            {t('startupExposesPath')}
          </span>
          <Tooltip label={t('helpHint')} text={t('startupExposesPathHint')} />
          <Switch
            labelledBy={exposeId}
            checked={draft.startupExposesPath}
            disabled={busy || draft.startupFile === ''}
            onChange={(next) => onChange({ ...draft, startupExposesPath: next })}
          />
        </div>
        {startupNeedsMigration && (
          <div className="field-switch">
            <span className="field-hint">{t('startupNeedsMigrationHint')}</span>
            {onMigrateStartup && (
              <button type="button" className="button small" onClick={onMigrateStartup}>
                {t('migrateStartup')}
              </button>
            )}
          </div>
        )}
      </div>
      <div className="field">
        <label htmlFor={noteId}>
          <FieldLabel label={t('note')} marker={t('optionalField')} />
        </label>
        <textarea
          id={noteId}
          rows={3}
          maxLength={2000}
          value={draft.note}
          placeholder={t('noteHint')}
          disabled={busy}
          onChange={(event) => onChange({ ...draft, note: event.target.value })}
        />
      </div>
      {notice}
      <div className="form-actions">
        {onCancel && (
          <button type="button" className="button" disabled={busy} onClick={onCancel}>
            {t('cancel')}
          </button>
        )}
        <button
          type="submit"
          className="button primary"
          disabled={busy || !configReady(draft.config) || !profileReady(draft.profile)}
        >
          {submitLabel}
        </button>
      </div>
    </form>
  )
}
