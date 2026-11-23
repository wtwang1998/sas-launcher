import { useId, useState, type FormEvent } from 'react'
import type { Config, ConfigRef, Language } from '../api'
import type { Translator } from '../i18n'
import { configReady } from '../lib/config'
import { ConfigChoice } from './ConfigChoice'
import { FieldLabel } from './form-fields'

export type ProjectDraft = { id: string | null; name: string; note: string; config: ConfigRef | null }

export const emptyProjectDraft: ProjectDraft = { id: null, name: '', note: '', config: null }

/**
 * One form for new projects, project edits, and the collapsed temporary launch.
 * The submitted draft already has its name and note trimmed.
 */
export function ProjectForm({ draft, configs, defaultConfig, language, t, busy, submitLabel, onChange, onSubmit, onCancel, onCreateConfiguration }: {
  draft: ProjectDraft
  configs: Config[]
  defaultConfig: Config | null
  language: Language
  t: Translator
  busy: boolean
  submitLabel: string
  onChange: (draft: ProjectDraft) => void
  onSubmit: (draft: ProjectDraft) => void
  onCancel?: () => void
  onCreateConfiguration?: () => void
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

  return <form className="editor" noValidate onSubmit={submit}>
    <div className="field">
      <label htmlFor={nameId}><FieldLabel label={t('name')} marker={t('requiredField')} /></label>
      <input id={nameId} name="name" value={draft.name} maxLength={100} autoFocus
        placeholder={t('example')} disabled={busy}
        aria-invalid={nameMissing || undefined} aria-describedby={nameMissing ? errorId : undefined}
        onChange={event => { setNameMissing(false); onChange({ ...draft, name: event.target.value }) }} />
      {nameMissing && <p className="field-error" id={errorId}>{t('nameRequired')}</p>}
    </div>
    <div className="field">
      <span className="field-title">{t('configSelection')}</span>
      <ConfigChoice value={draft.config} configs={configs} defaultConfig={defaultConfig} language={language} t={t}
        busy={busy} onCreateConfiguration={onCreateConfiguration}
        onChange={config => onChange({ ...draft, config })} />
    </div>
    <div className="field">
      <label htmlFor={noteId}><FieldLabel label={t('note')} marker={t('optionalField')} /></label>
      <textarea id={noteId} rows={3} maxLength={2000} value={draft.note} placeholder={t('noteHint')} disabled={busy}
        onChange={event => onChange({ ...draft, note: event.target.value })} />
    </div>
    <div className="form-actions">
      {onCancel && <button type="button" className="button" disabled={busy} onClick={onCancel}>{t('cancel')}</button>}
      <button type="submit" className="button primary" disabled={busy || !configReady(draft.config)}>{submitLabel}</button>
    </div>
  </form>
}
