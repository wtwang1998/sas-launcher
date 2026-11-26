import { FolderPlus, Play, Plus } from 'lucide-react'
import { useState } from 'react'
import type { Project, SasuserCandidate, View } from '../api'
import { NoticeBanner } from '../components/Notice'
import { ProjectCard } from '../components/ProjectCard'
import { type ProjectDraft, ProjectForm } from '../components/ProjectForm'
import { configsByRef } from '../lib/config'
import type { Notice } from '../lib/notice'
import { profileDraftFrom } from '../lib/profile'
import { useT } from '../i18n/provider'

export type ProjectsProps = {
  view: View
  draft: ProjectDraft | null
  profileCandidates: SasuserCandidate[]
  onBrowseProfile: () => void
  onBrowseStartupFile: () => void
  busy: boolean
  notice: Notice | null
  onDismissNotice: () => void
  onDraft: (draft: ProjectDraft | null) => void
  onNewProject: () => void
  onCreateConfiguration: (draft: ProjectDraft) => void
  onSaveProject: (draft: ProjectDraft) => Promise<void>
  onMigrateStartup: (projectId: string) => void
  onDeleteProject: (id: string) => Promise<void>
  onLaunchProject: (id: string) => Promise<void>
  onLaunchBatch: (ids: string[]) => Promise<void>
  /** Failures that belong to one project, rendered by that project's row. */
  projectNotices: Record<string, Notice>
  onDismissProjectNotice: (projectId: string) => void
}

export function Projects({
  view,
  draft,
  profileCandidates,
  onBrowseProfile,
  onBrowseStartupFile,
  busy,
  notice,
  onDismissNotice,
  onDraft,
  onNewProject,
  onCreateConfiguration,
  onSaveProject,
  onMigrateStartup,
  onDeleteProject,
  onLaunchProject,
  onLaunchBatch,
  projectNotices,
  onDismissProjectNotice,
}: ProjectsProps) {
  const t = useT()
  const [checked, setChecked] = useState<ReadonlySet<string>>(() => new Set())
  const selected = view.projects
    .filter((project) => checked.has(project.id))
    .map((project) => project.id)
  const allSelected = view.projects.length > 0 && selected.length === view.projects.length
  const editing = (project: Project) => draft?.id === project.id
  // Mirrors the backend guard (`project_running`): a running session for this
  // project already owns its sasuser folder and its log, so a second launch is
  // refused. Saying so on the row beats an error banner after the click.
  const running = new Set(
    view.sessions
      .filter((session) => session.status === 'running')
      .map((session) => session.projectId)
      .filter((id): id is string => id !== null),
  )

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
    await onLaunchBatch(selected)
  }

  const newProjectOpen = draft !== null && draft.id === null

  return (
    <>
      <header className="page-head">
        <div>
          <h1>{t('nav.projects')}</h1>
          <p>{t('nav.projectsHelp')}</p>
        </div>
        <button
          type="button"
          className="button primary"
          disabled={busy || newProjectOpen}
          onClick={() => onNewProject()}
        >
          <Plus size={17} aria-hidden="true" />
          {t('project.newProject')}
        </button>
      </header>

      {notice?.target === 'page' && (
        <NoticeBanner
          notice={notice}
          onDismiss={onDismissNotice}
          onMigrateStartup={onMigrateStartup}
          className="page-notice"
        />
      )}

      {draft && draft.id === null && (
        <section className="panel edit-panel">
          <h2>{t('project.newProject')}</h2>
          <ProjectForm
            draft={draft}
            configs={view.configs}
            defaultConfig={view.defaultConfig}
            profileCandidates={profileCandidates}
            busy={busy}
            submitLabel={t('project.saveProject')}
            onChange={onDraft}
            onCancel={() => onDraft(null)}
            onCreateConfiguration={() => onCreateConfiguration(draft)}
            onBrowseProfile={onBrowseProfile}
            onBrowseStartupFile={onBrowseStartupFile}
            onSubmit={(next) => void onSaveProject(next)}
            notice={
              notice?.target === 'form' ? (
                <NoticeBanner
                  notice={notice}
                  onDismiss={onDismissNotice}
                  onRetry={() => void onSaveProject(draft)}
                  onMigrateStartup={onMigrateStartup}
                  className="form-notice"
                />
              ) : null
            }
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
            {t('project.checkAll')}
          </label>
          <button
            type="button"
            className="button launch"
            disabled={busy || selected.length === 0}
            onClick={() => void launchSelected()}
          >
            <Play size={16} aria-hidden="true" />
            {t('project.launchSelected', { count: selected.length })}
          </button>
        </div>
      )}

      {view.projects.length === 0 && !draft && (
        <div className="empty panel">
          <span className="empty-icon">
            <FolderPlus size={31} strokeWidth={1.5} aria-hidden="true" />
          </span>
          <h2>{t('project.noProjects')}</h2>
          <p>{t('project.noProjectsHelp')}</p>
          <button type="button" className="button" disabled={busy} onClick={() => onNewProject()}>
            <FolderPlus size={17} aria-hidden="true" />
            {t('project.firstProject')}
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
            busy={busy}
            running={running.has(project.id)}
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
                startupFile: project.startupFile ?? '',
                startupExposesPath: project.startupExposesPath,
              })
            }
            onDelete={() => void onDeleteProject(project.id)}
            onLaunch={() => void onLaunchProject(project.id)}
            notice={
              projectNotices[project.id] ? (
                <NoticeBanner
                  notice={projectNotices[project.id]}
                  compact
                  onDismiss={() => onDismissProjectNotice(project.id)}
                  onMigrateStartup={onMigrateStartup}
                  className="card-notice"
                />
              ) : null
            }
          >
            {editing(project) && draft && (
              <ProjectForm
                draft={draft}
                configs={view.configs}
                defaultConfig={view.defaultConfig}
                profileCandidates={profileCandidates}
                busy={busy}
                submitLabel={t('project.saveProject')}
                onChange={onDraft}
                onCancel={() => onDraft(null)}
                onCreateConfiguration={() => onCreateConfiguration(draft)}
                onBrowseProfile={onBrowseProfile}
                onBrowseStartupFile={onBrowseStartupFile}
                startupNeedsMigration={
                  view.projects.find((project) => project.id === draft.id)?.startupNeedsMigration ??
                  false
                }
                onMigrateStartup={() => onMigrateStartup(draft.id ?? '')}
                onSubmit={(next) => void onSaveProject(next)}
                notice={
                  notice?.target === 'form' ? (
                    <NoticeBanner
                      notice={notice}
                      onDismiss={onDismissNotice}
                      onRetry={() => void onSaveProject(draft)}
                      onMigrateStartup={onMigrateStartup}
                      className="form-notice"
                    />
                  ) : null
                }
              />
            )}
          </ProjectCard>
        ))}
      </div>
    </>
  )
}
