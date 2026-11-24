import { getCurrentWindow } from '@tauri-apps/api/window'
import { open } from '@tauri-apps/plugin-dialog'
import {
  Activity,
  Archive,
  Check,
  CircleAlert,
  FolderOpen,
  Folders,
  Layers3,
  RefreshCw,
  Settings as SettingsIcon,
  SquareTerminal,
  X,
} from 'lucide-react'
import { useEffect, useRef, useState } from 'react'
import {
  api,
  type BatchResult,
  type ConfigBase,
  type Language,
  refKey,
  type SasuserCandidate,
  type Session,
  sameRef,
  type View,
} from './api'
import { useConfirmation } from './components/confirmation'
import { emptyProjectDraft, type ProjectDraft } from './components/ProjectForm'
import { browserLanguage, issueText, type MessageKey, translate } from './i18n'
import { onHostEvent } from './lib/host'
import { profileChoiceFor } from './lib/profile'
import { themeAttribute } from './lib/settings'
import { Configurations } from './pages/Configurations'
import { Diagnostics } from './pages/Diagnostics'
import { Projects } from './pages/Projects'
import { Settings as SettingsPage } from './pages/Settings'

type Page = 'projects' | 'configurations' | 'diagnostics' | 'settings'

const navigation = [
  { id: 'projects', label: 'projects', icon: Folders },
  { id: 'configurations', label: 'configurations', icon: Layers3 },
  { id: 'diagnostics', label: 'sessions', icon: Activity },
  { id: 'settings', label: 'settings', icon: SettingsIcon },
] as const

type Toast = { key: MessageKey; values?: Record<string, string | number> }

export function App() {
  const [view, setView] = useState<View | null>(null)
  const [page, setPage] = useState<Page>('projects')
  const [draft, setDraft] = useState<ProjectDraft | null>(null)
  const [profileCandidates, setProfileCandidates] = useState<SasuserCandidate[]>([])
  const [pendingDraft, setPendingDraft] = useState<ProjectDraft | null>(null)
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState<unknown>(null)
  const [toast, setToast] = useState<Toast | null>(null)
  const [fallbackLanguage] = useState<Language>(browserLanguage)
  const saving = useRef(false)
  const revision = useRef(0)
  const language = view?.language ?? fallbackLanguage
  const t = translate(language)
  const { confirm, confirmation } = useConfirmation(t)

  useEffect(() => {
    const title = translate(language)('title')
    document.documentElement.lang = language
    document.title = title
    try {
      void getCurrentWindow()
        .setTitle(title)
        .catch(() => undefined)
    } catch {
      // Not running inside a Tauri window (unit tests, plain browser preview).
    }
  }, [language])

  useEffect(() => {
    const theme = themeAttribute(view?.themePreference ?? 'system')
    if (theme) document.documentElement.dataset.theme = theme
    else delete document.documentElement.dataset.theme
  }, [view?.themePreference])

  useEffect(() => {
    let alive = true
    async function poll() {
      if (saving.current) return
      const current = revision.current
      try {
        const next = await api.snapshot()
        if (alive && next && current === revision.current && !saving.current) setView(next)
      } catch (failure) {
        if (alive && current === revision.current) setError(failure)
      }
    }
    void poll()
    const timer = window.setInterval(() => void poll(), 2800)
    return () => {
      alive = false
      window.clearInterval(timer)
    }
  }, [])

  async function mutate(action: () => Promise<View>, success?: MessageKey): Promise<View | null> {
    if (saving.current) return null
    saving.current = true
    revision.current += 1
    setBusy(true)
    setError(null)
    try {
      const next = await action()
      if (next) setView(next)
      if (success) setToast({ key: success })
      return next ?? null
    } catch (failure) {
      setError(failure)
      return null
    } finally {
      saving.current = false
      setBusy(false)
    }
  }

  async function launch(action: () => Promise<Session>, success: MessageKey) {
    if (saving.current) return
    saving.current = true
    revision.current += 1
    setBusy(true)
    setError(null)
    try {
      const session = await action()
      setToast({ key: success, values: { name: session.name } })
      const next = await api.snapshot()
      if (next) setView(next)
    } catch (failure) {
      setError(failure)
    } finally {
      saving.current = false
      setBusy(false)
    }
  }

  function navigate(next: Page) {
    setPage(next)
    setDraft(null)
    setPendingDraft(null)
    setError(null)
    setToast(null)
  }

  function beginConfigurationTrip(next: ProjectDraft) {
    setPendingDraft(next)
    setPage('configurations')
    setError(null)
    setToast(null)
  }

  function returnToDraft() {
    if (!pendingDraft) return
    // The parked draft may point at a configuration deleted while the trip was
    // in progress; never restore a dead reference.
    const alive =
      !pendingDraft.config ||
      (view?.configs ?? []).some((config) => sameRef(config.reference, pendingDraft.config))
    setDraft(alive ? pendingDraft : { ...pendingDraft, config: null })
    setPendingDraft(null)
    setPage('projects')
    setError(null)
    setToast(null)
  }

  async function saveProject(next: ProjectDraft) {
    const project = view?.projects.find((entry) => entry.id === next.id) ?? null
    const updated = await mutate(
      () =>
        api.saveProject(
          next.id,
          next.name.trim(),
          next.note.trim() || null,
          next.config,
          profileChoiceFor(project, next.profile),
        ),
      'projectSaved',
    )
    if (updated) setDraft(null)
  }

  /**
   * Open an empty form and load the folders SAS wrote before, so the first card
   * can already point at the layout the user tuned. Editing a draft afterwards
   * must not re-run this: it would reload the list on every keystroke and undo a
   * choice the user just made.
   */
  async function startNewProject() {
    setDraft({ ...emptyProjectDraft })
    let found: SasuserCandidate[] = []
    try {
      found = await api.sasuserCandidates(null)
    } catch (failure) {
      setError(failure)
    }
    setProfileCandidates(found)
    setDraft((current) => {
      // Only fill a draft that is still the untouched empty selection.
      if (!current || current.id !== null) return current
      if (current.profile.kind !== 'sasUser' || current.profile.path) return current
      const first = found[0]
      return {
        ...current,
        profile: first
          ? { kind: 'sasUser', path: first.path, layers: current.profile.layers }
          : { kind: 'fresh', layers: current.profile.layers },
      }
    })
  }

  /** A folder outside `My SAS Files` is still a valid source, but must be picked. */
  async function browseProfile() {
    try {
      const picked = await open({ directory: true, title: t('browseFolder') })
      if (typeof picked !== 'string' || !picked) return
      const found = await api.sasuserCandidates(picked)
      setProfileCandidates(found)
      setDraft((current) =>
        current
          ? {
              ...current,
              profile: { kind: 'sasUser', path: picked, layers: current.profile.layers },
            }
          : current,
      )
    } catch (failure) {
      setError(failure)
    }
  }

  async function deleteProject(id: string) {
    const name = view?.projects.find((project) => project.id === id)?.name
    confirm(
      t('deleteTitle'),
      t('deletePrompt', { name: name ?? '' }),
      t('deleteAction'),
      async () => {
        await mutate(() => api.deleteProject(id), 'projectDeleted')
      },
    )
  }

  async function launchProject(id: string) {
    await launch(() => api.launchProject(id), 'launched')
  }

  async function launchBatch(ids: string[]): Promise<BatchResult[]> {
    if (saving.current) return []
    saving.current = true
    revision.current += 1
    setBusy(true)
    setError(null)
    try {
      const results = await api.launchBatch(ids)
      const next = await api.snapshot()
      if (next) setView(next)
      return results
    } catch (failure) {
      setError(failure)
      return []
    } finally {
      saving.current = false
      setBusy(false)
    }
  }

  async function createConfiguration(base: ConfigBase, name: string | null) {
    const known = new Set((view?.configs ?? []).map((config) => refKey(config.reference)))
    const next = await mutate(() => api.createConfiguration(base, name), 'configurationCreated')
    if (!next) return
    const created = next.configs.find((config) => !known.has(refKey(config.reference)))
    const createdId = created?.reference.kind === 'user' ? created.reference.id : null
    if (createdId) {
      setPendingDraft((pending) =>
        pending ? { ...pending, config: { kind: 'user', id: createdId } } : pending,
      )
    }
  }

  async function updateConfiguration(id: string, encoding: string, locale: string) {
    await mutate(() => api.updateConfiguration(id, encoding, locale), 'configurationSaved')
  }

  async function deleteConfiguration(id: string) {
    const name = view?.configs.find((config) => refKey(config.reference) === `user:${id}`)?.name
    confirm(
      t('deleteConfigurationTitle'),
      t('deleteConfigurationDetail', { name: name ?? '' }),
      t('deleteAction'),
      async () => {
        await mutate(() => api.deleteConfiguration(id), 'configurationDeleted')
      },
    )
  }

  async function startConfigSetup(id: string) {
    await mutate(() => api.startConfigSetup(id), 'setupStarted')
  }

  async function finishConfigSetup(id: string, save: boolean) {
    await mutate(() => api.finishConfigSetup(id, save), save ? 'setupSaved' : 'setupDiscarded')
  }

  async function clearDiagnostics() {
    confirm(t('clearRecordsTitle'), t('clearRecordsDetail'), t('clearRecordsAction'), async () => {
      await mutate(() => api.clearDiagnostics(), 'recordsCleared')
    })
  }

  async function saveSasPath(path: string) {
    await mutate(() => api.selectInstallation(path), 'installationUpdated')
  }

  async function autoDetectSas() {
    await mutate(() => api.clearInstallation(), 'installationAutoDetected')
  }

  /**
   * Rust decides what a close means; only the "leave SAS running behind" answer
   * comes back here, so the confirmation copy stays in the dictionary.
   */
  const quitRequested = useRef<() => void>(() => undefined)
  useEffect(() => {
    quitRequested.current = () => {
      const count = view?.sessions.filter((session) => session.status === 'running').length ?? 0
      confirm(t('quitTitle'), t('quitMessage', { count }), t('quitAction'), async () => {
        await api.quit()
      })
    }
  })
  useEffect(() => onHostEvent('quit-requested', () => quitRequested.current()), [])

  function openLog(id: string) {
    void api.openLog(id).catch((failure) => setError(failure))
  }

  function openSasuser(id: string) {
    void api.openSasuser(id).catch((failure) => setError(failure))
  }

  function openArchive() {
    void api.openArchive().catch((failure) => setError(failure))
  }

  function pickExe() {
    void (async () => {
      try {
        const selected = await open({
          title: t('pickSas'),
          filters: [{ name: t('windowsProgram'), extensions: ['exe'] }],
        })
        if (typeof selected === 'string' && selected) {
          await mutate(() => api.selectInstallation(selected), 'installationUpdated')
        }
      } catch (failure) {
        setError(failure)
      }
    })()
  }

  return (
    <div className="app">
      <aside className="sidebar">
        <div className="brand">
          <span className="brand-icon">
            <SquareTerminal size={24} strokeWidth={1.9} aria-hidden="true" />
          </span>
          <span>
            {t('title')}
            {view && <small>v{view.version}</small>}
          </span>
        </div>
        <nav aria-label={t('navigation')}>
          {navigation.map(({ id, label, icon: Icon }) => (
            <button
              key={id}
              type="button"
              className={`nav-item ${page === id ? 'selected' : ''}`}
              aria-current={page === id ? 'page' : undefined}
              onClick={() => navigate(id)}
            >
              <Icon size={18} strokeWidth={1.8} aria-hidden="true" />
              {t(label)}
            </button>
          ))}
        </nav>
      </aside>

      <main>
        <div className="content">
          {error !== null && (
            <div className="banner error" role="alert">
              <CircleAlert size={18} aria-hidden="true" />
              <span>{issueText(language, error)}</span>
              <button
                type="button"
                className="icon-button"
                aria-label={t('closeError')}
                onClick={() => setError(null)}
              >
                <X size={16} />
              </button>
            </div>
          )}
          {toast && (
            <div className="banner success" role="status">
              <Check size={18} aria-hidden="true" />
              <span>{t(toast.key, toast.values)}</span>
              <button
                type="button"
                className="icon-button"
                aria-label={t('closeMessage')}
                onClick={() => setToast(null)}
              >
                <X size={16} />
              </button>
            </div>
          )}
          {view?.scanError && (
            <div className="banner warning">
              <CircleAlert size={18} aria-hidden="true" />
              <span>{issueText(language, view.scanError)}</span>
              <button type="button" className="button small" disabled={busy} onClick={pickExe}>
                <FolderOpen size={15} aria-hidden="true" />
                {t('chooseExe')}
              </button>
            </div>
          )}
          {view?.defaultError && !view.scanError && (
            <div className="banner warning">
              <CircleAlert size={18} aria-hidden="true" />
              <span>{issueText(language, view.defaultError)}</span>
              <button
                type="button"
                className="button small"
                disabled={busy}
                onClick={() => void mutate(() => api.refresh(), 'refreshed')}
              >
                <RefreshCw size={15} aria-hidden="true" />
                {t('refresh')}
              </button>
            </div>
          )}
          {view?.archived && (
            <div className="banner info">
              <Archive size={18} aria-hidden="true" />
              <span>{t('archiveNotice', { path: view.archived })}</span>
              <button type="button" className="button small" disabled={busy} onClick={openArchive}>
                {t('openArchive')}
              </button>
            </div>
          )}
          {!view && error === null && (
            <p className="muted" role="status">
              {t('discovering')}
            </p>
          )}

          {view && page === 'projects' && (
            <Projects
              view={view}
              draft={draft}
              profileCandidates={profileCandidates}
              onBrowseProfile={() => void browseProfile()}
              language={language}
              t={t}
              busy={busy}
              onNewProject={() => void startNewProject()}
              onDraft={setDraft}
              onCreateConfiguration={beginConfigurationTrip}
              onSaveProject={saveProject}
              onDeleteProject={deleteProject}
              onLaunchProject={launchProject}
              onLaunchBatch={launchBatch}
            />
          )}

          {view && page === 'configurations' && (
            <Configurations
              view={view}
              language={language}
              t={t}
              busy={busy}
              returnDraft={pendingDraft ? returnToDraft : null}
              onCreate={createConfiguration}
              onUpdate={updateConfiguration}
              onDelete={deleteConfiguration}
              onStartSetup={startConfigSetup}
              onFinishSetup={finishConfigSetup}
            />
          )}

          {view && page === 'diagnostics' && (
            <Diagnostics
              view={view}
              language={language}
              t={t}
              busy={busy}
              onClear={clearDiagnostics}
              onOpenLog={openLog}
              onOpenSasuser={openSasuser}
            />
          )}

          {view && page === 'settings' && (
            <SettingsPage
              view={view}
              t={t}
              busy={busy}
              onTheme={(preference) => void mutate(() => api.setTheme(preference))}
              onLanguage={(preference) => void mutate(() => api.setLanguage(preference))}
              onSavePath={saveSasPath}
              onBrowse={pickExe}
              onAutoDetect={autoDetectSas}
              onStartup={(enabled) => void mutate(() => api.setStartup(enabled))}
              onStartMinimized={(enabled) => void mutate(() => api.setStartMinimized(enabled))}
              onCloseToTray={(enabled) => void mutate(() => api.setCloseToTray(enabled))}
            />
          )}
        </div>
      </main>
      {confirmation}
    </div>
  )
}
