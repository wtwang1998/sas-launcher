import { useEffect, useId, useRef, useState, type ReactNode, type Ref } from 'react'
import { ArrowLeft, ArrowUpRight, Check, ChevronRight, CircleAlert, FolderOpen, Pencil, Plus, RefreshCw, SquareTerminal, Trash2 } from 'lucide-react'
import { api, keyOf, sameKey, sourceFor, type Config, type ConfigKey, type View } from './api'
import { configurationLabel, encodingLabel, encodingOf, localeOf, localeLabel, sasLabel } from './config-options'
import type { Confirm } from './confirmation'
import { ConfigurationSelect, selectionFor, type ConfigurationSelection } from './ConfigurationSelect'
import { FieldLabel } from './form-fields'
import { issueText, type MessageKey, type Translator } from './i18n'

export type Act = (action: () => Promise<View | void>, success?: MessageKey) => Promise<void>
type Props = { view: View; configs: Config[]; busy: boolean; t: Translator; act: Act; confirm: Confirm }

function configurationName(t: Translator, view: View, config: Config) {
  return `${encodingLabel(t, config.encoding)} · ${localeLabel(t, view.language, config)} · ${configurationLabel(t, config, view.configs)}`
}

function detectedSource(view: View, config: Config) {
  return view.sources.find(source => source.keys.some(key => sameKey(key, config.key)))
}

function restoreDefaults({ view, t, confirm, act }: Omit<Props, 'configs' | 'busy'>, config: Config, onboarding: boolean) {
  const label = t('useFactory')
  const configuration = configurationName(t, view, config)
  confirm(label, t('restoreFactoryPrompt', { configuration }), label,
    () => act(() => api.factory(config.key, onboarding), 'factoryDone'))
}

function restoreUserSettings({ view, t, confirm, act }: Omit<Props, 'configs' | 'busy'>, config: Config, directory: string, onboarding: boolean) {
  const label = t('importExisting')
  const configuration = configurationName(t, view, config)
  confirm(label, t('restoreUserPrompt', { configuration, directory }), label,
    () => act(() => api.import(config.key, directory, onboarding), 'importDone'))
}

export function Onboarding({ view, configs, busy, t, act, confirm }: Props) {
  const [target, setTarget] = useState<string | null>(null)
  const [sourcePath, setSourcePath] = useState<string | null>(null)
  const upgrade = view.onboarding === 'upgrade'
  const targets = configs.filter(config => !config.error && !sourceFor(view, config))
  const preferred = targets.find(config => view.defaultKey && sameKey(config.key, view.defaultKey))
    ?? (targets.length === 1 ? targets[0] : undefined)
  const config = target === null ? preferred : targets.find(config => keyOf(config.key) === target)
  const candidate = sourcePath === null
    ? (config ? detectedSource(view, config) : undefined)
      ?? (view.sources.length === 1 ? view.sources[0] : undefined)
    : view.sources.find(source => source.directory === sourcePath)

  return <section className="panel edit-panel onboarding" aria-labelledby="onboarding-title">
    <div className="onboarding-header"><span className="card-icon"><FolderOpen size={19} /></span>
      <div>
        <h2 id="onboarding-title">{t(upgrade ? 'upgradeTitle' : 'onboarding')}</h2>
        <p className="muted">{t(upgrade ? 'upgradeHelp' : 'onboardingHelp')}</p>
      </div>
    </div>
    <div className="editor">
      <label><FieldLabel label={t('targetConfig')} marker={t('requiredField')} />
        <select aria-label={t('targetConfig')} required value={config ? keyOf(config.key) : ''} onChange={event => { setTarget(event.target.value); setSourcePath(null) }} disabled={busy}>
          <option value="" disabled>{t('chooseConfig')}</option>
          {targets.map(config => <option value={keyOf(config.key)} key={keyOf(config.key)}>
            {configurationName(t, view, config)}{view.defaultKey && sameKey(view.defaultKey, config.key) ? ` (${t('defaultOption')})` : ''}
          </option>)}
        </select>
      </label>
      {view.sources.length ? <>
        <label>{t('chooseSource')}
          <select aria-label={t('chooseSource')} aria-describedby="onboarding-source-hint" value={candidate?.directory ?? ''} onChange={event => setSourcePath(event.target.value)} disabled={busy}>
            <option value="" disabled>{t('chooseSourcePlaceholder')}</option>
            {view.sources.map(source => <option key={source.directory} value={source.directory}>{source.directory}</option>)}
          </select>
          <small className="field-hint" id="onboarding-source-hint">{t('sourceNeededForRestore')}</small>
        </label>
        {candidate && <div className="detected-files"><strong>{t('detectedFiles')}</strong><ul>{candidate.files.map(file => <li key={file}><code>{file}</code></li>)}</ul></div>}
      </> : <p className="muted">{t('onboardingNoSource')}</p>}
    </div>
    <div className="template-actions">
      {candidate && <button className="button danger-outline" disabled={busy || !config} onClick={() => {
        if (config) restoreUserSettings({ view, t, confirm, act }, config, candidate.directory, true)
      }}>{t('importExisting')}</button>}
      <button className="button danger-outline" disabled={busy || !config} onClick={() => {
        if (config) restoreDefaults({ view, t, confirm, act }, config, true)
      }}>{t('useFactory')}</button>
      <button className="button" disabled={busy} onClick={() => void act(api.dismissOnboarding)}>{t('later')}</button>
    </div>
  </section>
}


const unknown = 'unknown'
const encodingFilterValue = (config: Config) => encodingOf(config) || unknown
function TemplateRow({ view, busy, t, act, confirm, config, open, toggle, target, returnAction, summaryRef, management, editor }: Omit<Props, 'configs'> & {
  config: Config
  open: boolean
  toggle: () => void
  target: boolean
  returnAction?: ReactNode
  summaryRef?: Ref<HTMLButtonElement>
  management: ReactNode
  editor?: ReactNode
}) {
  const detailsId = useId()
  const source = sourceFor(view, config)
  const candidate = detectedSource(view, config)
  const setup = view.sessions.find(session => sameKey(session.key, config.key) && session.templateSetup && session.status !== 'unknown')
  const name = configurationName(t, view, config)

  return <article className={`panel config-card ${target ? 'highlight' : ''}`}>
    <h3>
      <button type="button" className="config-summary" ref={summaryRef} aria-expanded={open} aria-controls={detailsId} onClick={toggle}>
        <ChevronRight className={`disclosure ${open ? 'expanded' : ''}`} size={16} aria-hidden="true" />
        <span className="config-summary-name">{name}</span>
        <span className={`template-status ${source && !config.error ? 'good' : ''}`}>
          {source && !config.error ? <Check size={14} /> : <span className="status-dot" />}
          {t(config.error ? 'configUnavailable' : source?.kind ?? 'factory')}
        </span>
      </button>
    </h3>
    <div className="config-body" id={detailsId} hidden={!open}>
      {open && <>
        {config.error && <p className="muted">{issueText(view.language, config.error)}</p>}
        {view.defaultKey && sameKey(view.defaultKey, config.key) && <p className="muted">{t('installationDefault')}</p>}
        {target && returnAction}
        {management}
        {editor}
        {!config.error && <h4 className="personal-settings-title">{t('personalSettings')}</h4>}
        {!config.error && !setup && <div className="source-choices">
          {candidate && <p className="muted"><strong>{t('detectedSource')}</strong><br /><span>{candidate.directory}</span></p>}
        </div>}
        {!config.error && <div className="template-actions">
          {setup?.status === 'running' ? <span className="muted">{t('setupStarted')}</span>
            : setup?.status === 'ready' ? <>
              <span className="muted">{t(setup.exitCode ? 'setupFailed' : 'setupReady')}</span>
              <button className="button small" disabled={busy} onClick={() => void act(() => api.finishSetup(setup.id, false), 'templateDiscarded')}>{t('discard')}</button>
              <button className="button primary small" disabled={busy || !!setup.exitCode} onClick={() => void act(() => api.finishSetup(setup.id, true), 'templateSaved')}>{t('saveTemplate')}</button>
            </> : <div className="setup-tooltip">
              <button className="button primary small" disabled={busy} aria-describedby={detailsId + '-sas-tip'}
                onClick={() => void act(() => api.setup(config.key), 'setupStarted')}><ArrowUpRight size={15} />{t('openSasSettings')}</button>
              <span className="setup-tip" role="tooltip" id={detailsId + '-sas-tip'}>{t('openSasSettingsTip')}</span>
            </div>}
        </div>}
        {!config.error && !setup && <section className="template-restore" aria-labelledby={detailsId + '-restore'}>
          <h4 id={detailsId + '-restore'}><CircleAlert size={15} aria-hidden="true" />{t('restoreSettings')}</h4>
          <div className="template-actions">
            <button className="button small danger-outline" disabled={busy}
              onClick={() => restoreDefaults({ view, t, confirm, act }, config, false)}>{t('useFactory')}</button>
            <button className="button small danger-outline" disabled={busy || !candidate}
              aria-describedby={!candidate ? detailsId + '-restore-hint' : undefined} onClick={() => {
                if (candidate) restoreUserSettings({ view, t, confirm, act }, config, candidate.directory, false)
              }}>{t('importExisting')}</button>
          </div>
          {!candidate && <p className="muted" id={detailsId + '-restore-hint'}>{t('restoreUserUnavailable')}</p>}
        </section>}
        <details className="config-details">
          <summary>{t('details')} <ChevronRight className="disclosure" size={14} /></summary>
          <dl className="diagnostic">
            {config.base ? <><dt>{t('baseConfiguration')}</dt><dd>{config.base.cfg}</dd></> : <><dt>{t('configCode')}</dt><dd>{config.code}</dd></>}
            <dt>{t('locale')}</dt><dd>{config.locale || t('notDetected')}</dd>
            <dt>{t('sasInterfaceLanguage')}</dt><dd>{sasLabel(t, view.language, config)}</dd>
            <dt>{t('cfgPath')}</dt><dd>{config.file ?? config.key.cfg}</dd>
            <dt>{t('defaultFolder')}</dt><dd>{config.defaultProfile || t('notDetected')}{config.defaultProfile && <span className="declared-folder-help">{t('declaredFolderHelp')}</span>}</dd>
            {source?.kind === 'imported' && <><dt>{t('sourcePath')}</dt><dd>{source.directory}</dd></>}
          </dl>
        </details>
      </>}
    </div>
  </article>
}

type ConfigurationDraft = {
  anchor: string
  id: string | null
  name: string
  originalBase: ConfigKey | null
  selection: ConfigurationSelection
  repair: boolean
}

export function SettingsTemplates({ view, configs, busy, t, act, confirm, pickExe, target, onReturn, returnLabel }: Props & {
  pickExe: () => void
  target?: Config
  onReturn?: () => void
  returnLabel?: string | null
}) {
  const targetKey = target ? keyOf(target.key) : ''
  const [encoding, setEncoding] = useState(target ? encodingFilterValue(target) : '')
  const [locale, setLocale] = useState(target ? localeOf(target) : '')
  const [expanded, setExpanded] = useState(targetKey)
  const [draft, setDraft] = useState<ConfigurationDraft | null>(null)
  const originals = configs.filter(config => !config.customId)
  const collator = new Intl.Collator(view.language, { numeric: true })
  const byEncoding = (value: string) => encodingLabel(t, value === unknown ? null : value)
  const byLocale = (config: Config) => localeLabel(t, view.language, config)
  const sorted = [...configs].sort((left, right) =>
    collator.compare(byEncoding(encodingFilterValue(left)), byEncoding(encodingFilterValue(right)))
    || collator.compare(byLocale(left), byLocale(right))
    || collator.compare(left.code, right.code))
  const encodings = [...new Set(sorted.map(encodingFilterValue))]
  const localeLabels = new Map(sorted.map(config => [localeOf(config), byLocale(config)]))
  const locales = [...localeLabels].sort((left, right) => collator.compare(left[1], right[1]))
  const activeEncoding = encodings.includes(encoding) ? encoding : ''
  const activeLocale = localeLabels.has(locale) ? locale : ''
  const matches = sorted.filter(config =>
    (!activeEncoding || encodingFilterValue(config) === activeEncoding)
    && (!activeLocale || localeOf(config) === activeLocale))

  function startEditing(config?: Config) {
    const base = config?.base
    const current = base ? originals.find(original => sameKey(original.key, base)) : config
    setDraft({
      anchor: config ? keyOf(config.key) : '',
      id: config?.customId ?? null,
      name: config?.customName ?? '',
      originalBase: base ?? null,
      selection: selectionFor(current?.error ? undefined : current),
      repair: !!config?.error,
    })
  }

  function saveConfiguration(base: Config) {
    if (!draft) return
    const save = () => act(async () => {
      const next = await api.saveConfiguration(draft.id, draft.name, base.key)
      setDraft(null)
      return next
    }, 'configurationSaved')
    if (draft.id && (draft.repair || (draft.originalBase && !sameKey(draft.originalBase, base.key)))) {
      confirm(t('changeConfiguration'), t('changeConfigurationDetail'), t('saveConfiguration'), save)
    } else {
      void save()
    }
  }

  const selectedBase = originals.find(config => !config.error && keyOf(config.key) === draft?.selection.config)
  const editor = draft && <form className="editor configuration-editor" onSubmit={event => {
    event.preventDefault()
    if (selectedBase) saveConfiguration(selectedBase)
  }}>
    <h3>{t(draft.id ? 'editConfiguration' : 'newConfiguration')}</h3>
    <label><FieldLabel label={t('configurationName')} marker={t('optionalField')} />
      <input autoFocus maxLength={100} value={draft.name} disabled={busy}
        placeholder={t('configurationNameHint')} onChange={event => setDraft({ ...draft, name: event.target.value })} />
    </label>
    <ConfigurationSelect selection={draft.selection} onChange={selection => setDraft({ ...draft, selection })}
      configs={originals} language={view.language} defaultKey={view.defaultKey} busy={busy} t={t} base />
    <div className="form-actions">
      <button type="button" className="button" disabled={busy} onClick={() => setDraft(null)}>{t('cancel')}</button>
      <button className="button primary" disabled={!selectedBase || busy}>{t('saveConfiguration')}</button>
    </div>
  </form>

  const returnAction = onReturn && <button className="button small template-return" onClick={onReturn} disabled={busy}>
    <ArrowLeft size={15} />{returnLabel}
  </button>
  const targetIsOpen = expanded === targetKey && matches.some(config => targetKey === keyOf(config.key))

  const targetSummary = useRef<HTMLButtonElement>(null)
  const focusedTarget = useRef(false)
  useEffect(() => {
    if (!targetIsOpen || focusedTarget.current) return
    const frame = window.requestAnimationFrame(() => {
      if (!targetSummary.current) return
      focusedTarget.current = true
      targetSummary.current.focus({ preventScroll: true })
      targetSummary.current.scrollIntoView({ block: 'start' })
    })
    return () => window.cancelAnimationFrame(frame)
  }, [targetIsOpen])


  return <>
    {!targetIsOpen && returnAction}
    <section className="panel install">
      <div className="install-heading">
        <span className="card-icon"><SquareTerminal size={20} /></span><div><h2>{t('installation')}</h2><p title={view.exe ?? ''}>{view.exe ?? t('noInstallation')}</p></div>
      </div>
      <div className="card-actions">
        <button className="button small" disabled={busy} onClick={pickExe}><FolderOpen size={15} />{t('chooseProgram')}</button>
        <button className="icon-button" aria-label={t('refresh')} title={t('refresh')} disabled={busy} onClick={() => void act(api.refresh, 'refreshed')}><RefreshCw size={17} /></button>
      </div>
    </section>
    {target && !configs.some(config => sameKey(config.key, target.key)) &&
      <div className="banner warning" role="status"><CircleAlert size={17} /><span>{t('missingConfig')}</span></div>}
    <div className="section-title configuration-heading">
      <h2>{t('allConfigurations')}</h2><span role="status">{t('templateFilterCount', { shown: matches.length, total: configs.length })}</span>
      <button className="button primary small" disabled={busy || !!draft || !originals.some(config => !config.error)}
        onClick={() => startEditing()}><Plus size={16} />{t('newConfiguration')}</button>
    </div>
    {draft?.anchor === '' && <section className="panel edit-panel">{editor}</section>}
    {configs.length > 0 && <div className="template-filters">
      <label>{t('filterEncoding')}<select aria-label={t('filterEncoding')} value={activeEncoding} disabled={!!draft} onChange={event => setEncoding(event.target.value)}>
        <option value="">{t('allEncodings')}</option>
        {encodings.map(value => <option key={value} value={value}>{byEncoding(value)}</option>)}
      </select></label>
      <label>{t('locale')}<select aria-label={t('locale')} value={activeLocale} disabled={!!draft} onChange={event => setLocale(event.target.value)}>
        <option value="">{t('allLocales')}</option>
        {locales.map(([value, label]) => <option key={value} value={value}>{label}</option>)}
      </select></label>
      {(activeEncoding || activeLocale) && <button className="button small" disabled={!!draft} onClick={() => { setEncoding(''); setLocale('') }}>{t('clearTemplateFilters')}</button>}
    </div>}
    {configs.length > 0 && matches.length === 0 && <p className="muted no-template-matches">{t('noTemplateMatches')}</p>}
    <div className="template-list">
      {matches.map(config => {
        const key = keyOf(config.key)
        const customId = config.customId
        const names = view.projects.filter(project => sameKey(project.key, config.key)).map(project => project.name)
        const setupActive = view.sessions.some(session => session.templateSetup && sameKey(session.key, config.key))
        const management = <div className="configuration-management">
          <div className="template-actions">
            <button className="button small" disabled={busy || !!draft || setupActive || (!customId && !!config.error)}
              onClick={() => startEditing(config)}>{customId ? <Pencil size={15} /> : <Plus size={15} />}
              {t(customId ? 'editConfiguration' : 'copyConfiguration')}
            </button>
            {customId && <button className="button small danger-outline" disabled={busy || !!draft || setupActive || names.length > 0}
              onClick={() => confirm(t('deleteConfiguration'), t('deleteConfigurationDetail', { name: configurationLabel(t, config, configs) }),
                t('deleteAction'), () => act(() => api.deleteConfiguration(customId), 'configurationDeleted'))}>
              <Trash2 size={15} />{t('deleteConfiguration')}
            </button>}
          </div>
          {customId && names.length > 0 && <p className="muted">{t('configurationInUseHint', { names: names.join(view.language === 'en' ? ', ' : '、') })}</p>}
        </div>
        return <TemplateRow key={key} view={view} busy={busy} t={t} act={act} confirm={confirm} config={config} open={expanded === key}
          management={management} editor={draft?.anchor === key ? editor : undefined}
          target={targetKey === key} returnAction={targetKey === key ? returnAction : undefined} summaryRef={targetKey === key ? targetSummary : undefined} toggle={() => { if (!draft) setExpanded(expanded === key ? '' : key) }} />
      })}
    </div>
    {view.sessions.filter(session => session.templateSetup && session.status === 'ready' && !configs.some(config => sameKey(config.key, session.key))).map(session =>
      <div className="banner warning" key={session.id}><CircleAlert size={17} /><span>{t('setupMissing')}</span>
        <button className="button small" disabled={busy} onClick={() => void act(() => api.finishSetup(session.id, false), 'templateDiscarded')}>{t('discard')}</button>
        <button className="button small" disabled={busy || !!session.exitCode} onClick={() => void act(() => api.finishSetup(session.id, true), 'templateSaved')}>{t('saveTemplate')}</button>
      </div>
    )}
  </>
}
