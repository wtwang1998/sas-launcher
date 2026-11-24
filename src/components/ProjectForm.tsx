import { type FormEvent, type ReactNode, useId, useState } from 'react'
import type { Config, ConfigRef, Language, SasuserCandidate } from '../api'
import type { Translator } from '../i18n'
import { configReady } from '../lib/config'
import { emptyProfileDraft, type ProfileDraft, profileReady } from '../lib/profile'
import { ConfigChoice } from './ConfigChoice'
import { FieldLabel } from './form-fields'
import { UserFolderChoice } from './UserFolderChoice'

export type ProjectDraft = {
  id: string | null
  name: string
  note: string
  config: ConfigRef | null
  profile: ProfileDraft
}

export const emptyProjectDraft: ProjectDraft = {
  id: null,
  name: '',
  note: '',
  config: null,
  profile: emptyProfileDraft,
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
  /** The last save's outcome, shown next to the buttons it belongs to. */
  notice?: ReactNode
}) {
  const nameId = useId()
  const noteId = useId()
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
