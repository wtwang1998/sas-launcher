import { Activity, Check, ChevronRight, CircleAlert, FolderOpen, FolderPlus, Folders, Languages, Layers3, Pencil, Play, Plus, Settings2, SquareTerminal, Trash2, X, Zap } from 'lucide-react'
import { open } from '@tauri-apps/plugin-dialog'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { useEffect, useId, useRef, useState, type FormEvent } from 'react'
import { api, keyOf, sameKey, type BatchResult, type Config, type Language, type Preference, type Project, type Session, type View } from './api'
import { configurationLabel, encodingLabel, encodingOf, localeOf, localeLabel, sasLabel, orderedConfigs } from './config-options'
import { browserLanguage, issueText, translate, type MessageKey, type Translator } from './i18n'
import { useConfirmation } from './confirmation'
import { ConfigurationSelect, emptySelection, selectionFor, type ConfigurationSelection } from './ConfigurationSelect'
import { FieldLabel } from './form-fields'
import { Onboarding, SettingsTemplates, type Act } from './SettingsTemplates'

type SourcePage = 'projects' | 'quick'
type Page = SourcePage | 'templates'
type Draft = ConfigurationSelection & { id: string | null; name: string }
const emptyDraft: Draft = { id: null, name: '', ...emptySelection }
const navigation = [
  { id: 'projects', help: 'projectsHelp', icon: Folders },
  { id: 'quick', help: 'quickHelp', icon: Zap },
  { id: 'templates', help: 'templatesHelp', icon: Layers3 },
] as const

function ProjectEditor({ draft, setDraft, onCancel, onSubmit, configs, view, busy, t }: {
  draft: Draft
  setDraft: (draft: Draft) => void
  onCancel?: () => void
  onSubmit: (event: FormEvent<HTMLFormElement>) => void
  configs: Config[]
  view: View
  busy: boolean
  t: Translator
}) {
  const selected = configs.some(config => !config.error && keyOf(config.key) === draft.config
    && encodingOf(config) === draft.encoding && localeOf(config) === draft.locale)
  const quick = !onCancel
  const nameHint = useId()

  return <form onSubmit={onSubmit} className="editor">
    <label><FieldLabel label={t('name')} marker={t(quick ? 'optionalField' : 'requiredField')} /><input name="name" autoFocus={!quick} value={draft.name} onChange={event => setDraft({ ...draft, name: event.target.value })} placeholder={t('example')} maxLength={100} required={!quick} aria-describedby={quick ? nameHint : undefined} />
      {quick && <small className="field-hint" id={nameHint}>{t('quickUnnamedHelp', { name: t('temporaryProject') })}</small>}
    </label>
    <ConfigurationSelect selection={draft} onChange={selection => setDraft({ ...draft, ...selection })}
      configs={configs} language={view.language} defaultKey={view.defaultKey} busy={busy} t={t} />
    <div className="form-actions">
      {onCancel && <button type="button" className="button" onClick={onCancel}>{t('cancel')}</button>}
      <button className="button primary" disabled={busy || (!quick && !draft.name.trim()) || !selected}>
        {quick && <Play size={16} />}{t(quick ? 'launch' : 'saveProject')}
      </button>
    </div>
  </form>
}

function SessionRow({ session, t }: { session: Session; t: Translator }) {
  return <details className="session">
    <summary><Activity size={16} /><strong>{session.title}</strong><span>{t(session.status)}</span><ChevronRight className="disclosure" size={15} /></summary>
    <dl className="diagnostic">
      <dt>{t('sasProgram')}</dt><dd>{session.key.exe}</dd>
      <dt>{t('sasUser')}</dt><dd>{session.profile}</dd>
      <dt>{t('arguments')}</dt><dd><code>{session.args.map(argument => JSON.stringify(argument)).join(' ')}</code></dd>
    </dl>
  </details>
}

export function App() {
  const [view, setView] = useState<View | null>(null)
  const [page, setPage] = useState<Page>('projects')
  const [draft, setDraft] = useState<Draft | null>(null)
  const startingDraft = useRef<Draft | null>(null)
  const [quick, setQuick] = useState<Draft>(emptyDraft)
  const [checked, setChecked] = useState<Set<string>>(() => new Set())
  const [batchFailures, setBatchFailures] = useState<BatchResult[]>([])
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState<{ value: unknown } | null>(null)
  const [message, setMessage] = useState<{ key: MessageKey; title?: string } | null>(null)
  const [templateVisit, setTemplateVisit] = useState<{ config: Config; from: SourcePage; scrollTop: number } | null>(null)
  const saving = useRef(false)
  const revision = useRef(0)
  const [fallbackLanguage] = useState<Language>(browserLanguage)
  const language = view?.language ?? fallbackLanguage
  const t = translate(language)
  const { confirm, confirmation } = useConfirmation(t)

  useEffect(() => {
    const title = translate(language)('title')
    document.documentElement.lang = language
    document.title = title
    void getCurrentWindow().setTitle(title).catch(() => undefined)
  }, [language])

  useEffect(() => {
    let alive = true
    async function poll() {
      if (saving.current) return
      const current = revision.current
      try {
        const next = await api.snapshot()
        if (alive && current === revision.current && !saving.current) setView(next)
      } catch (error) {
        if (alive && current === revision.current) setError({ value: error })
      }
    }
    void poll()
    const timer = window.setInterval(() => void poll(), 2800)
    return () => { alive = false; window.clearInterval(timer) }
  }, [])

  const act: Act = async (action, success) => {
    if (saving.current) return
    saving.current = true
    revision.current += 1
    setBusy(true)
    setError(null)
    setMessage(null)
    try {
      const next = await action()
      if (next) {
        setView(next)
        if (success) setMessage({ key: success })
      }
    } catch (error) {
      setError({ value: error })
    } finally {
      saving.current = false
      setBusy(false)
    }
  }

  function navigate(next: Page) {
    setDraft(null)
    setPage(next)
    setTemplateVisit(null)
    setError(null)
    setMessage(null)
  }

  const configs = view ? orderedConfigs(view.configs, view.systemLanguage, view.defaultKey) : []
  const projects = view?.projects ?? []
  const sessions = view?.sessions ?? []
  const selectedIds = projects.filter(project => checked.has(project.id)).map(project => project.id)
  const allSelected = projects.length > 0 && selectedIds.length === projects.length
  const available = configs.filter(config => !config.error).length
  const running = sessions.filter(session => session.status === 'running').length

  function openProjectEditor(next: Draft) {
    if (draft && draft.id === next.id) return
    function beginEditing() {
      startingDraft.current = next
      setDraft(next)
    }

    const initial = startingDraft.current
    const dirty = draft && initial && (draft.name !== initial.name || draft.encoding !== initial.encoding || draft.locale !== initial.locale || draft.config !== initial.config)
    if (dirty) {
      confirm(t('discardProjectChanges'), t('discardProjectChangesDetail'), t('discard'), beginEditing)
    } else {
      beginEditing()
    }
  }

  function editProject(project: Project) {
    const config = configs.find(config => sameKey(config.key, project.key) && !config.error)
    openProjectEditor({ id: project.id, name: project.name, ...selectionFor(config) })
  }

  function pickExe() {
    void act(async () => {
      const path = await open({ title: t('pickSas'), filters: [{ name: t('windowsProgram'), extensions: ['exe'] }] })
      if (path) return api.selectInstallation(path)
    }, 'installationUpdated')
  }

  function saveDraft(event: FormEvent<HTMLFormElement>) {
    event.preventDefault()
    const config = configs.find(config => keyOf(config.key) === draft?.config)
    if (!draft || !config) return
    void act(async () => {
      const next = await api.save(draft.id, draft.name, config.key)
      setDraft(null)
      return next
    }, 'projectSaved')
  }

  function openTemplate(config: Config) {
    setTemplateVisit({ config, from: page === 'quick' ? 'quick' : 'projects', scrollTop: window.scrollY })
    setPage('templates')
    setError(null)
    setMessage(null)
  }

  function returnFromTemplate() {
    if (!templateVisit) return
    const { from, scrollTop } = templateVisit
    setPage(from)
    window.requestAnimationFrame(() => window.scrollTo(0, scrollTop))
    setTemplateVisit(null)
    setError(null)
    setMessage(null)
  }

  function launch(name: string, config: Config, id: string | null) {
    void act(async () => {
      const session = await api.launch(name, config.key, id)
      setMessage({ key: 'launched', title: session.title })
      return api.snapshot()
    })
  }

  function toggle(id: string) {
    setChecked(current => {
      const next = new Set(current)
      if (next.has(id)) next.delete(id)
      else next.add(id)
      return next
    })
  }

  const projectEditor = draft && view && <ProjectEditor draft={draft} setDraft={setDraft}
    onCancel={() => setDraft(null)} onSubmit={saveDraft} configs={configs} view={view} busy={busy} t={t} />

  return <div className="app">
    <aside className="sidebar">
      <div className="brand"><span className="brand-icon"><SquareTerminal size={24} strokeWidth={1.9} /></span><span>{t('title')}<small>Windows · SAS 9.4</small></span></div>
      <nav aria-label={t('navigation')}>
        {navigation.map(({ id, icon: Icon }) => <button key={id} className={`nav-item ${page === id ? 'selected' : ''}`} aria-current={page === id ? 'page' : undefined} onClick={() => navigate(id)}>
          <Icon size={18} strokeWidth={1.8} />{t(id)}{page === id && <ChevronRight size={15} className="nav-arrow" />}
        </button>)}
      </nav>
      <div className="sidebar-bottom">
        <div className="health"><span className={`status-dot ${available ? 'green' : ''}`} />{available ? t('available', { count: available }) : t('waiting')}</div>
        <button className="text-button" onClick={() => navigate('templates')}><Settings2 size={15} />{t('manageInstallation')}</button>
        <label className="language-picker"><Languages size={15} role="presentation" /><span>{t('language')}</span>
          <select aria-label={t('language')} value={view?.languagePreference ?? 'system'} disabled={busy || !view}
            onChange={event => { const preference = event.target.value as Preference; void act(() => api.language(preference)) }}>
            <option value="system">{t('system')}</option><option value="zh-Hans">简体中文</option><option value="en">English</option>
          </select>
        </label>
      </div>
    </aside>
    <main>
      <header className="topline"><span>SAS / {t(page)}</span><span className="running"><Activity size={15} />{t('active', { count: running })}</span></header>
      <div className="content">
        <header className="page-head">
          <div><h1>{t(page)}</h1><p>{t(navigation.find(item => item.id === page)!.help)}</p></div>
          {page === 'projects' && <button className="button primary" disabled={!available || busy || draft?.id === null} onClick={() => openProjectEditor(emptyDraft)}><Plus size={17} />{t('newProject')}</button>}
        </header>
        {error && <div className="banner error" role="alert"><CircleAlert size={18} /><span>{issueText(language, error.value)}</span><button className="icon-button" aria-label={t('closeError')} onClick={() => setError(null)}><X size={16} /></button></div>}
        {message && <div className="banner success" role="status"><Check size={18} /><span>{t(message.key, { title: message.title ?? '' })}</span><button className="icon-button" aria-label={t('closeMessage')} onClick={() => setMessage(null)}><X size={16} /></button></div>}
        {view?.scanError && <div className="banner warning"><CircleAlert size={18} /><span>{issueText(language, view.scanError)}</span><button className="button small" disabled={busy} onClick={pickExe}><FolderOpen size={15} />{t('chooseExe')}</button></div>}
        {!view && !error && <p className="muted" role="status">{t('discovering')}</p>}
        {view?.onboarding && !templateVisit && <Onboarding key={view.onboarding} view={view} configs={configs} busy={busy} t={t} act={act} confirm={confirm} />}

        {page === 'projects' && view && <>
          {draft?.id === null && <section className="panel edit-panel">
            <h2>{t('newProject')}</h2>
            {projectEditor}
          </section>}
          {projects.length === 0 && !draft && <div className="empty panel">
            <span className="empty-icon"><Folders size={31} strokeWidth={1.5} /></span>
            <h2>{t('noProjects')}</h2>
            <p>{t('noProjectsHelp')}</p>
            <button className="button" disabled={!available || busy} onClick={() => openProjectEditor(emptyDraft)}><FolderPlus size={17} />{t('firstProject')}</button>
          </div>}
          {projects.length > 0 && <div className="batch-bar">
            <label className="project-checkbox"><input type="checkbox" checked={allSelected} disabled={busy}
              ref={input => { if (input) input.indeterminate = !allSelected && selectedIds.length > 0 }}
              onChange={() => setChecked(allSelected ? new Set() : new Set(projects.map(project => project.id)))} />{t('checkAll')}</label>
            <button className="button launch" disabled={busy || selectedIds.length === 0} onClick={() => void act(async () => {
              setBatchFailures([])
              const results = await api.batch(selectedIds)
              setBatchFailures(results.filter(result => result.status === 'failed'))
              return api.snapshot()
            })}><Play size={16} />{t('launchSelected', { count: selectedIds.length })}</button>
          </div>}
          {batchFailures.length > 0 && <div className="banner error batch-failures" role="alert">
            <CircleAlert size={18} />
            <div><strong>{t('batchFailures')}</strong>
              <ul>{batchFailures.map(result => <li key={result.id}>
                {result.name ?? t('unavailableProject')} · {issueText(language, result.reason)}
              </li>)}</ul>
            </div>
            <button className="icon-button" aria-label={t('closeError')} onClick={() => setBatchFailures([])}><X size={16} /></button>
          </div>}
          <div className="cards">
            {projects.map(project => {
              const config = configs.find(config => sameKey(config.key, project.key) && !config.error)
              const editing = draft?.id === project.id
              const editorId = `project-editor-${project.id}`
              return <article className="panel project-card" key={project.id}>
                <div className="project-summary">
                  <label className="project-checkbox"><input type="checkbox" aria-label={t('selectNamed', { name: project.name })} checked={checked.has(project.id)} disabled={busy} onChange={() => toggle(project.id)} /></label>
                  <span className="card-icon"><Folders size={20} strokeWidth={1.8} /></span>
                  <div className="card-info"><h2>{project.name}</h2>
                    <div className="card-meta">{config ? <>
                      <span className="chip">{encodingLabel(t, config.encoding)}<span className="chip-divider" />{localeLabel(t, language, config)}</span>
                      <span className="muted">{configurationLabel(t, config, configs)} · {sasLabel(t, language, config)}</span>
                    </> : <span className="missing">{t('missingConfig')}</span>}</div>
                  </div>
                  <div className="card-actions">
                    <button className="icon-button" aria-label={t('editNamed', { name: project.name })} aria-expanded={editing} aria-controls={editing ? editorId : undefined} disabled={busy} onClick={() => editProject(project)}><Pencil size={16} /></button>
                    <button className="icon-button delete" aria-label={t('deleteNamed', { name: project.name })} disabled={busy} onClick={() => confirm(t('deleteTitle'), t('deletePrompt', { name: project.name }), t('deleteAction'), () => act(async () => {
                      const next = await api.remove(project.id)
                      if (draft?.id === project.id) setDraft(null)
                      setChecked(current => new Set([...current].filter(id => id !== project.id)))
                      return next
                    }, 'projectDeleted'))}><Trash2 size={16} /></button>
                    {config && <>
                      <button className="icon-button" aria-label={t('manageNamedConfiguration', { name: project.name })}
                        disabled={busy} onClick={() => openTemplate(config)}><Settings2 size={16} /></button>
                      <button className="button small launch" disabled={busy} onClick={() => launch(project.name, config, project.id)}>
                        <Play size={14} />{t('launch')}
                      </button>
                    </>}
                  </div>
                </div>
                {editing && <section id={editorId} className="inline-project-editor">
                  <h3>{t('editProject')}</h3>
                  {projectEditor}
                </section>}
              </article>
            })}
          </div>
        </>}

        {page === 'quick' && view && <section className="panel edit-panel quick-panel">
          <ProjectEditor draft={quick} setDraft={setQuick} onSubmit={event => {
            event.preventDefault()
            const config = configs.find(config => keyOf(config.key) === quick.config)
            if (config) launch(quick.name, config, null)
          }} configs={configs} view={view} busy={busy} t={t} />
        </section>}

        {page === 'templates' && view && <SettingsTemplates view={view} configs={configs} busy={busy} t={t} act={act} confirm={confirm} pickExe={pickExe} target={templateVisit?.config} onReturn={templateVisit ? returnFromTemplate : undefined} returnLabel={templateVisit && t(templateVisit.from === 'quick' ? 'backToQuick' : 'backToProjects')} />}

        {(sessions.length > 0 || !!view?.notices.length) && <section className="session-section">
          <div className="section-title"><h2>{t('sessions')}</h2></div>
          {view?.notices.map((notice, index) => <p key={index} className="session-notice">{issueText(language, notice)}</p>)}
          {sessions.map(session => <SessionRow key={session.id} session={session} t={t} />)}
        </section>}
      </div>
    </main>
    {confirmation}
  </div>
}
