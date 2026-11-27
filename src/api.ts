import { invoke } from '@tauri-apps/api/core'

export type Issue = { code: string; detail?: string }
export type Language = 'en' | 'ja' | 'zh-Hans' | 'zh-Hant' | 'ko' | 'de'
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
/** Which layers of the user folder a project snapshot carries. */
export type ProfileLayers = { interface: boolean; environment: boolean; templates: boolean }
export type ProfileOrigin =
  | { kind: 'sasUser'; path: string }
  | { kind: 'configSnapshot'; config: string }
  | { kind: 'fresh' }
/** What a project's snapshot holds now; `files: 0` means nothing captured yet. */
export type ProjectProfile = {
  origin: ProfileOrigin | null
  layers: ProfileLayers
  files: number
  /** Catalogs the declared layers need but the snapshot does not hold. */
  missing: number
}
/** What the form asks the backend to do with a project's snapshot. */
export type ProfileChoice =
  | { kind: 'keep' }
  | { kind: 'sasUser'; path: string; layers: ProfileLayers }
  | { kind: 'configSnapshot'; config: string; layers: ProfileLayers }
  | { kind: 'fresh'; layers: ProfileLayers }
/** A version folder under `Documents\My SAS Files` that holds catalogs. */
/** Per-layer count of the whitelisted catalogs a folder holds. */
export type LayerCounts = { interface: number; environment: number; templates: number }
export type SasuserCandidate = {
  path: string
  version: string
  files: number
  present: LayerCounts
  modified: number
}
export type Project = {
  id: string
  name: string
  note: string | null
  config: ConfigRef | null
  profile: ProjectProfile | null
  /** A project's own autoexec file SAS runs at launch; null when it has none. */
  startupFile: string | null
  /** Whether the launch also tells the session where that file is. */
  startupExposesPath: boolean
  /** Whether the script still reads the variable the session cannot fill. */
  startupNeedsMigration: boolean
}
export type SessionStage = 'exe' | 'cfg' | 'profile' | 'spawn' | 'started'
export type SessionStatus = 'running' | 'exited' | 'setup' | 'cleanupPending' | 'unknown'
/** The preference layers a launch actually seeded, not the ones it asked for. */
export type ProfileSeed = { files: number; layers: ProfileLayers }
export type Chain = {
  label: string
  thinCfg: string | null
  factoryCfg: string | null
  encoding: string | null
  locale: string | null
  sasuser: string
  profile: ProfileSeed | null
  log: string | null
}
export type Session = {
  id: string
  name: string
  kind: 'project' | 'setup'
  configId: string | null
  /** The project this session belongs to; null for a settings session. */
  projectId: string | null
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
  version: string
  /** The commit this build came from, `-dirty` when the tree was modified. */
  commit: string
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
  sasuserCandidates: (extra: string | null) =>
    invoke<SasuserCandidate[]>('sasuser_candidates', { extra }),
  refresh: () => invoke<View>('refresh_configs'),
  selectInstallation: (path: string) => invoke<View>('select_installation', { path }),
  setLanguage: (preference: Preference) => invoke<View>('set_language', { preference }),
  clearInstallation: () => invoke<View>('clear_installation'),
  setTheme: (preference: ThemePreference) => invoke<View>('set_theme', { preference }),
  setCloseToTray: (enabled: boolean) => invoke<View>('set_close_to_tray', { enabled }),
  setStartup: (enabled: boolean) => invoke<View>('set_startup', { enabled }),
  setStartMinimized: (enabled: boolean) => invoke<View>('set_start_minimized', { enabled }),
  quit: () => invoke<void>('quit'),
  saveProject: (
    id: string | null,
    name: string,
    note: string | null,
    config: ConfigRef | null,
    profile: ProfileChoice | null,
    startupFile: string | null,
    startupExposesPath: boolean,
  ) =>
    invoke<View>('save_project', {
      id,
      name,
      note,
      config,
      profile,
      startupFile,
      startupExposesPath,
    }),
  deleteProject: (id: string) => invoke<View>('delete_project', { id }),
  migrateStartupFile: (id: string) => invoke<View>('migrate_startup_file', { id }),
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
  openRepository: () => invoke<void>('open_repository'),
  openIssues: () => invoke<void>('open_issues'),
  openRelease: (tag: string) => invoke<void>('open_release', { tag }),
}
