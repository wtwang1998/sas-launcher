import type { Config, Session, View } from '../api'

/** A built-in row: the name is the factory NLS directory code. */
export function builtIn(
  code: string,
  encoding: string | null,
  locale: string | null,
  patch: Partial<Config> = {},
): Config {
  return {
    reference: { kind: 'builtIn', code },
    name: code,
    encoding,
    locale,
    uiLanguage: null,
    preset: 'verified',
    derivedFrom: null,
    hasProfile: false,
    error: null,
    ...patch,
  }
}

/** A user configuration derived from `u8` by default. */
export function userConfig(
  id: string,
  name: string,
  encoding: string | null,
  locale: string | null,
  patch: Partial<Config> = {},
): Config {
  return {
    reference: { kind: 'user', id },
    name,
    encoding,
    locale,
    uiLanguage: 'zh-Hans',
    preset: 'verified',
    derivedFrom: 'u8',
    hasProfile: true,
    error: null,
    ...patch,
  }
}

export function makeSession(patch: Partial<Session> & { id: string }): Session {
  return {
    name: '会话 A',
    kind: 'project',
    configId: null,
    status: 'running',
    exitCode: null,
    startedAt: Date.UTC(2026, 10, 23, 4, 0),
    stage: 'started',
    argv: ['C:\\SAS\\sas.exe', '-CONFIG', 'C:\\SAS\\nls\\u8\\sasv9.cfg'],
    chain: {
      label: 'u8',
      thinCfg: 'C:\\data\\configurations\\cfg-1\\sasv9.cfg',
      factoryCfg: 'C:\\SAS\\nls\\u8\\sasv9.cfg',
      encoding: 'UTF-8',
      locale: 'zh_CN',
      sasuser: 'C:\\data\\sessions\\s1\\sasuser',
      log: 'C:\\data\\sessions\\s1\\SAS.LOG',
    },
    sasuserWritable: true,
    errors: 0,
    warnings: 0,
    hints: [],
    failure: null,
    ...patch,
  }
}

export function makeView(patch: Partial<View> = {}): View {
  return {
    exe: 'C:\\SAS\\sas.exe',
    defaultConfig: builtIn('zh', 'EUC-CN', 'zh_CN'),
    defaultError: null,
    scanError: null,
    configs: [],
    projects: [],
    sessions: [],
    archived: null,
    languagePreference: 'zh-Hans',
    language: 'zh-Hans',
    systemLanguage: 'zh-CN',
    themePreference: 'system',
    closeToTray: true,
    startMinimized: false,
    autostart: false,
    ...patch,
  }
}
