import { invoke } from '@tauri-apps/api/core'

export type Language = 'en' | 'zh-Hans'
export type Preference = 'system' | Language
export type Issue = { code: string; detail?: string }
export type ConfigKey = { exe: string; cfg: string }
export type ConfigRef = { kind: 'builtIn'; exe: string; code: string } | { kind: 'custom'; id: string }
export type Config = {
  reference: ConfigRef
  key: ConfigKey
  code: string
  customId: string | null
  customName: string | null
  base: ConfigKey | null
  file: string | null
  encoding: string | null
  locale: string | null
  uiLanguage: string | null
  isDefault: boolean
  defaultProfile: string | null
  error: Issue | null
}

export type Project = { id: string; name: string; note: string | null; config: ConfigRef | null }
export type Session = {
  id: string
  key: ConfigKey
  name: string
  title: string
  profile: string
  args: string[]
  templateSetup: boolean
  status: 'running' | 'ready' | 'unknown' | 'cleanupPending'
  exitCode: number | null
}
export type Source = { kind: 'existing' | 'factory' | 'customized' } | { kind: 'imported'; directory: string }
export type Candidate = { directory: string; files: string[]; keys: ConfigKey[] }
export type BatchResult = { id: string; name: string | null; status: 'started' | 'skipped' | 'failed'; reason: Issue | null }
export type View = {
  exe: string | null
  configs: Config[]
  defaultKey: ConfigKey | null
  defaultRef: ConfigRef | null
  defaultConfig: Config | null
  defaultError: Issue | null
  sources: Candidate[]
  projects: Project[]
  templates: { key: ConfigKey; source: Source }[]
  sessions: Session[]
  scanError: Issue | null
  notices: Issue[]
  systemLanguage: string
  languagePreference: Preference
  language: Language
}

export const keyOf = (key: ConfigKey) => JSON.stringify([key.exe, key.cfg])
export const sameKey = (a: ConfigKey, b: ConfigKey) => a.exe === b.exe && a.cfg === b.cfg
export const sameRef = (a: ConfigRef | null | undefined, b: ConfigRef | null | undefined) =>
  a?.kind === b?.kind && (a?.kind === 'custom' ? a.id === (b as Extract<ConfigRef, { kind: 'custom' }> | undefined)?.id
    : a?.kind === 'builtIn' && a.exe === (b as Extract<ConfigRef, { kind: 'builtIn' }> | undefined)?.exe
      && a.code === (b as Extract<ConfigRef, { kind: 'builtIn' }> | undefined)?.code)
export const sourceFor = (view: View, config: Config) =>
  view.templates.find(({ key }) => sameKey(key, config.key))?.source

export const api = {
  snapshot: () => invoke<View>('snapshot'),
  refresh: () => invoke<View>('refresh_configs'),
  selectInstallation: (path: string) => invoke<View>('select_installation', { path }),
  language: (preference: Preference) => invoke<View>('set_language', { preference }),
  save: (id: string | null, name: string, note: string, config: ConfigRef | null, encoding: string, locale: string) =>
    invoke<View>('save_project', { id, name, note: note || null, config, encoding, locale }),
  saveConfiguration: (id: string | null, name: string, base: ConfigKey, encoding: string, locale: string) =>
    invoke<View>('save_configuration', { id, name, base, encoding, locale }),
  deleteConfiguration: (id: string) => invoke<View>('delete_configuration', { id }),
  remove: (id: string) => invoke<View>('delete_project', { id }),
  import: (key: ConfigKey, directory: string) => invoke<View>('import_template', { key, directory }),
  factory: (key: ConfigKey) => invoke<View>('use_factory', { key }),
  launch: (name: string, config: ConfigRef | null, id: string | null) => invoke<Session>('launch_project', { name, config, id }),
  batch: (ids: string[]) => invoke<BatchResult[]>('launch_batch', { ids }),
  setup: (key: ConfigKey) => invoke<View>('start_template_setup', { key }),
  finishSetup: (id: string, save: boolean) => invoke<View>('finish_template_setup', { id, save }),
}
