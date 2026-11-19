import { invoke } from '@tauri-apps/api/core'

export type Language = 'en' | 'zh-Hans'
export type Preference = 'system' | Language
export type Issue = { code: string; detail?: string }
export type ConfigKey = { exe: string; cfg: string }
export type Config = {
  key: ConfigKey
  code: string
  customId: string | null
  customName: string | null
  base: ConfigKey | null
  file: string | null
  encoding: string | null
  locale: string | null
  uiLanguage: string | null
  defaultProfile: string | null
  error: Issue | null
}

export type Project = { id: string; name: string; key: ConfigKey }
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
  sources: Candidate[]
  projects: Project[]
  templates: { key: ConfigKey; source: Source }[]
  sessions: Session[]
  scanError: Issue | null
  notices: Issue[]
  onboarding: 'first' | 'upgrade' | null
  systemLanguage: string
  languagePreference: Preference
  language: Language
}

export const keyOf = (key: ConfigKey) => JSON.stringify([key.exe, key.cfg])
export const sameKey = (a: ConfigKey, b: ConfigKey) => a.exe === b.exe && a.cfg === b.cfg
export const sourceFor = (view: View, config: Config) =>
  view.templates.find(({ key }) => sameKey(key, config.key))?.source

export const api = {
  snapshot: () => invoke<View>('snapshot'),
  refresh: () => invoke<View>('refresh_configs'),
  selectInstallation: (path: string) => invoke<View>('select_installation', { path }),
  language: (preference: Preference) => invoke<View>('set_language', { preference }),
  dismissOnboarding: () => invoke<View>('dismiss_onboarding'),
  save: (id: string | null, name: string, key: ConfigKey) => invoke<View>('save_project', { id, name, key }),
  saveConfiguration: (id: string | null, name: string, base: ConfigKey) => invoke<View>('save_configuration', { id, name, base }),
  deleteConfiguration: (id: string) => invoke<View>('delete_configuration', { id }),
  remove: (id: string) => invoke<View>('delete_project', { id }),
  import: (key: ConfigKey, directory: string, onboarding = false) => invoke<View>('import_template', { key, directory, onboarding }),
  factory: (key: ConfigKey, onboarding = false) => invoke<View>('use_factory', { key, onboarding }),
  launch: (name: string, key: ConfigKey, id: string | null) => invoke<Session>('launch_project', { name, key, id }),
  batch: (ids: string[]) => invoke<BatchResult[]>('launch_batch', { ids }),
  setup: (key: ConfigKey) => invoke<View>('start_template_setup', { key }),
  finishSetup: (id: string, save: boolean) => invoke<View>('finish_template_setup', { id, save }),
}
