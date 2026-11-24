import { CircleAlert, FolderPlus, Play, Plus, X } from 'lucide-react'
import { useState } from 'react'
import type { BatchResult, Language, Project, SasuserCandidate, View } from '../api'
import { ProjectCard } from '../components/ProjectCard'
import { type ProjectDraft, ProjectForm } from '../components/ProjectForm'
import { issueText, type Translator } from '../i18n'
import { configsByRef } from '../lib/config'
import { profileDraftFrom } from '../lib/profile'

export type ProjectsProps = {
  view: View
  draft: ProjectDraft | null
  profileCandidates: SasuserCandidate[]
  onBrowseProfile: () => void
  language: Language
  t: Translator
  busy: boolean
  onDraft: (draft: ProjectDraft | null) => void
  onNewProject: () => void
  onCreateConfiguration: (draft: ProjectDraft) => void
  onSaveProject: (draft: ProjectDraft) => Promise<void>
  onDeleteProject: (id: string) => Promise<void>
  onLaunchProject: (id: string) => Promise<void>
  onLaunchBatch: (ids: string[]) => Promise<BatchResult[]>
}

export function Projects({
  view,
  draft,
  profileCandidates,
  onBrowseProfile,
  language,
  t,
  busy,
  onDraft,
  onNewProject,
  onCreateConfiguration,
  onSaveProject,
  onDeleteProject,
  onLaunchProject,
  onLaunchBatch,
}: ProjectsProps) {
  const [checked, setChecked] = useState<ReadonlySet<string>>(() => new Set())
  const [failures, setFailures] = useState<BatchResult[]>([])
  const selected = view.projects
    .filter((project) => checked.has(project.id))
    .map((project) => project.id)
  const allSelected = view.projects.length > 0 && selected.length === view.projects.length
  const editing = (project: Project) => draft?.id === project.id

  function toggle(id: string) {
    setChecked((current) => {
      const next = new Set(current)
      if (next.has(id)) next.delete(id)
      else next.add(id)
      return next
    })
  }

  function toggleAll() {
    setChecked(allSelected ? new Set() : new Set(view.projects.map((project) => project.id)))
  }

  async function launchSelected() {
    const results = await onLaunchBatch(selected)
    setFailures(results.filter((result) => result.status === 'failed'))
  }

  const newProjectOpen = draft !== null && draft.id === null

  return (
    <>
      <header className="page-head">
        <div>
          <h1>{t('projects')}</h1>
          <p>{t('projectsHelp')}</p>
        </div>
        <button
          type="button"
          className="button primary"
          disabled={busy || newProjectOpen}
          onClick={() => onNewProject()}
        >
          <Plus size={17} aria-hidden="true" />
          {t('newProject')}
        </button>
      </header>

      {draft && draft.id === null && (
        <section className="panel edit-panel">
          <h2>{t('newProject')}</h2>
          <ProjectForm
            draft={draft}
            configs={view.configs}
            defaultConfig={view.defaultConfig}
            profileCandidates={profileCandidates}
            language={language}
            t={t}
            busy={busy}
            submitLabel={t('saveProject')}
            onChange={onDraft}
            onCancel={() => onDraft(null)}
            onCreateConfiguration={() => onCreateConfiguration(draft)}
            onBrowseProfile={onBrowseProfile}
            onSubmit={(next) => void onSaveProject(next)}
          />
        </section>
      )}

      {view.projects.length > 0 && (
        <div className="batch-bar">
          <label className="project-checkbox">
            <input
              type="checkbox"
              checked={allSelected}
              disabled={busy}
              ref={(input) => {
                if (input) input.indeterminate = !allSelected && selected.length > 0
              }}
              onChange={toggleAll}
            />
            {t('checkAll')}
          </label>
          <button
            type="button"
            className="button launch"
            disabled={busy || selected.length === 0}
            onClick={() => void launchSelected()}
          >
            <Play size={16} aria-hidden="true" />
            {t('launchSelected', { count: selected.length })}
          </button>
        </div>
      )}

      {failures.length > 0 && (
        <div className="banner error batch-failures" role="alert">
          <CircleAlert size={18} aria-hidden="true" />
          <div>
            <strong>{t('batchFailures')}</strong>
            <ul>
              {failures.map((result) => (
                <li key={result.id}>
                  {result.name ?? t('unavailableProject')} · {issueText(language, result.reason)}
                </li>
              ))}
            </ul>
          </div>
          <button
            type="button"
            className="icon-button"
            aria-label={t('closeError')}
            onClick={() => setFailures([])}
          >
            <X size={16} />
          </button>
        </div>
      )}

      {view.projects.length === 0 && !draft && (
        <div className="empty panel">
          <span className="empty-icon">
            <FolderPlus size={31} strokeWidth={1.5} aria-hidden="true" />
          </span>
          <h2>{t('noProjects')}</h2>
          <p>{t('noProjectsHelp')}</p>
          <button type="button" className="button" disabled={busy} onClick={() => onNewProject()}>
            <FolderPlus size={17} aria-hidden="true" />
            {t('firstProject')}
          </button>
        </div>
      )}

      <div className="cards">
        {view.projects.map((project) => (
          <ProjectCard
            key={project.id}
            project={project}
            config={
              project.config
                ? (configsByRef(view.configs, project.config) ?? null)
                : view.defaultConfig
            }
            language={language}
            t={t}
            busy={busy}
            checked={checked.has(project.id)}
            editing={editing(project)}
            onCheck={() => toggle(project.id)}
            onEdit={() =>
              onDraft({
                id: project.id,
                name: project.name,
                note: project.note ?? '',
                config: project.config,
                profile: profileDraftFrom(project.profile),
              })
            }
            onDelete={() => void onDeleteProject(project.id)}
            onLaunch={() => void onLaunchProject(project.id)}
          >
            {editing(project) && draft && (
              <ProjectForm
                draft={draft}
                configs={view.configs}
                defaultConfig={view.defaultConfig}
                profileCandidates={profileCandidates}
                language={language}
                t={t}
                busy={busy}
                submitLabel={t('saveProject')}
                onChange={onDraft}
                onCancel={() => onDraft(null)}
                onCreateConfiguration={() => onCreateConfiguration(draft)}
                onBrowseProfile={onBrowseProfile}
                onSubmit={(next) => void onSaveProject(next)}
              />
            )}
          </ProjectCard>
        ))}
      </div>
    </>
  )
}
