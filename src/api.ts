import { invoke } from '@tauri-apps/api/core'

export type Issue = { code: string; detail?: string }
export type Language = 'en' | 'zh-Hans'
export type Preference = 'system' | Language
export type ThemePreference = 'system' | 'dark' | 'light'
export type PresetStatus = 'verified' | 'failed' | 'unverified'
export type ConfigRef = { kind: 'builtIn'; code: string } | { kind: 'user'; id: string }
export type ConfigBase =
  | { kind: 'default' }
  | { kind: 'builtIn'; code: string }
  | { kind: 'user'; id: string }
export type Config = {
  reference: ConfigRef
  name: string
  encoding: string | null
  locale: string | null
  uiLanguage: string | null
  preset: PresetStatus
  derivedFrom: string | null
  hasProfile: boolean
  error: Issue | null
}
export type Project = { id: string; name: string; note: string | null; config: ConfigRef | null }
export type SessionStage = 'exe' | 'cfg' | 'profile' | 'spawn' | 'started'
export type SessionStatus = 'running' | 'exited' | 'setup' | 'cleanupPending' | 'unknown'
export type Chain = {
  label: string
  thinCfg: string | null
  factoryCfg: string | null
  encoding: string | null
  locale: string | null
  sasuser: string
  log: string | null
}
export type Session = {
  id: string
  name: string
  kind: 'project' | 'setup'
  configId: string | null
  status: SessionStatus
  exitCode: number | null
  startedAt: number
  stage: SessionStage
  argv: string[]
  chain: Chain
  sasuserWritable: boolean
  errors: number
  warnings: number
  hints: Issue[]
  failure: Issue | null
}
export type BatchResult = {
  id: string
  name: string | null
  status: 'started' | 'skipped' | 'failed'
  reason: Issue | null
}
export type View = {
  exe: string | null
  defaultConfig: Config | null
  defaultError: Issue | null
  scanError: Issue | null
  configs: Config[]
  projects: Project[]
  sessions: Session[]
  archived: string | null
  languagePreference: Preference
  language: Language
  systemLanguage: string
  themePreference: ThemePreference
  closeToTray: boolean
  trayHintSeen: boolean
  startMinimized: boolean
  autostart: boolean
}

/** Reference equality for the three-state project configuration pointer. */
export const sameRef = (a: ConfigRef | null | undefined, b: ConfigRef | null | undefined) =>
  a?.kind === b?.kind &&
  (a?.kind === 'user'
    ? a.id === (b as Extract<ConfigRef, { kind: 'user' }> | undefined)?.id
    : a?.kind === 'builtIn' &&
      a.code === (b as Extract<ConfigRef, { kind: 'builtIn' }> | undefined)?.code)

/** Stable identity of a configuration reference, used as a React key. */
export const refKey = (ref: ConfigRef | null | undefined) =>
  ref ? (ref.kind === 'user' ? `user:${ref.id}` : `builtIn:${ref.code}`) : 'default'

export const api = {
  snapshot: () => invoke<View>('snapshot'),
  refresh: () => invoke<View>('refresh_configs'),
  selectInstallation: (path: string) => invoke<View>('select_installation', { path }),
  setLanguage: (preference: Preference) => invoke<View>('set_language', { preference }),
  clearInstallation: () => invoke<View>('clear_installation'),
  setTheme: (preference: ThemePreference) => invoke<View>('set_theme', { preference }),
  setCloseToTray: (enabled: boolean) => invoke<View>('set_close_to_tray', { enabled }),
  setStartup: (enabled: boolean) => invoke<View>('set_startup', { enabled }),
  setStartMinimized: (enabled: boolean) => invoke<View>('set_start_minimized', { enabled }),
  hideToTray: () => invoke<View>('hide_to_tray'),
  quit: () => invoke<void>('quit'),
  saveProject: (id: string | null, name: string, note: string | null, config: ConfigRef | null) =>
    invoke<View>('save_project', { id, name, note, config }),
  deleteProject: (id: string) => invoke<View>('delete_project', { id }),
  createConfiguration: (base: ConfigBase, name: string | null) =>
    invoke<View>('create_configuration', { base, name }),
  updateConfiguration: (id: string, encoding: string, locale: string) =>
    invoke<View>('update_configuration', { id, encoding, locale }),
  deleteConfiguration: (id: string) => invoke<View>('delete_configuration', { id }),
  startConfigSetup: (id: string) => invoke<View>('start_config_setup', { id }),
  finishConfigSetup: (id: string, save: boolean) =>
    invoke<View>('finish_config_setup', { id, save }),
  launchProject: (id: string) => invoke<Session>('launch_project', { id }),
  launchBatch: (ids: string[]) => invoke<BatchResult[]>('launch_batch', { ids }),
  clearDiagnostics: () => invoke<View>('clear_diagnostics'),
  openLog: (id: string) => invoke<void>('open_log', { id }),
  openSasuser: (id: string) => invoke<void>('open_sasuser', { id }),
  openArchive: () => invoke<void>('open_archive'),
}
