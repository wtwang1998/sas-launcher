import { Folders, Pencil, Play, Trash2 } from 'lucide-react'
import type { ReactNode } from 'react'
import type { Config, Language, Project } from '../api'
import type { Translator } from '../i18n'
import { configLabel } from '../lib/config'
import { Chips } from './Chips'

export function ProjectCard({ project, config, language, t, busy, checked, editing, onCheck, onEdit, onDelete, onLaunch, children }: {
  project: Project
  config: Config | null
  language: Language
  t: Translator
  busy: boolean
  checked: boolean
  editing: boolean
  onCheck: () => void
  onEdit: () => void
  onDelete: () => void
  onLaunch: () => void
  children?: ReactNode
}) {
  return <article className="panel project-card">
    <div className="project-summary">
      <input type="checkbox" className="project-checkbox" aria-label={t('selectNamed', { name: project.name })}
        checked={checked} disabled={busy} onChange={onCheck} />
      <span className="card-icon"><Folders size={20} strokeWidth={1.8} aria-hidden="true" /></span>
      <div className="card-info">
        <h2>{project.name}</h2>
        <div className="card-meta">
          {config
            ? <>
              <span className="chip chip-name">{configLabel(t, config)}</span>
              <Chips config={config} language={language} t={t} />
            </>
            : <span className="missing">{t('missingConfig')}</span>}
        </div>
        {project.note && <p className="project-note">{project.note}</p>}
      </div>
      <div className="card-actions">
        <button type="button" className="icon-button" aria-label={t('editNamed', { name: project.name })}
          aria-expanded={editing} disabled={busy} onClick={onEdit}><Pencil size={16} aria-hidden="true" /></button>
        <button type="button" className="icon-button delete" aria-label={t('deleteNamed', { name: project.name })}
          disabled={busy} onClick={onDelete}><Trash2 size={16} aria-hidden="true" /></button>
        <button type="button" className="button small launch" disabled={busy} onClick={onLaunch}>
          <Play size={14} aria-hidden="true" />{t('launch')}
        </button>
      </div>
    </div>
    {editing && <div className="inline-project-editor">{children}</div>}
  </article>
}
