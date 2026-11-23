import {
  sameRef,
  type Chain,
  type Config,
  type ConfigRef,
  type Language,
  type PresetStatus,
  type Project,
} from '../api'
import type { MessageKey, Translator } from '../i18n'

/** Display name of a configuration: the user-given name, or the factory directory code. */
export function configLabel(t: Translator, config: Config): string {
  const name = config.name?.trim()
  if (name) return name
  return config.reference.kind === 'builtIn' ? config.reference.code : t('unnamedConfiguration')
}

export function presetLabel(t: Translator, preset: PresetStatus): string {
  return preset === 'failed'
    ? t('presetFailed')
    : preset === 'unverified'
      ? t('presetUnverified')
      : t('presetVerified')
}

const collator = new Intl.Collator('zh-Hans', { numeric: true, sensitivity: 'base' })

/** Sort key: the display name, falling back to the factory code, unnamed last. */
function sortName(config: Config): string {
  const name = config.name?.trim()
  if (name) return name
  return config.reference.kind === 'builtIn' ? config.reference.code : '\uFFFF'
}

function byName(configs: Config[]): Config[] {
  return [...configs].sort((a, b) => collator.compare(sortName(a), sortName(b)))
}

export function groupConfigs(configs: Config[]): { users: Config[]; builtIns: Config[] } {
  return {
    users: byName(configs.filter((config) => config.reference.kind === 'user')),
    builtIns: byName(configs.filter((config) => config.reference.kind === 'builtIn')),
  }
}

/** User configurations first, then built-in ones, each group ordered by name. */
export function sortConfigs(configs: Config[]): Config[] {
  const { users, builtIns } = groupConfigs(configs)
  return [...users, ...builtIns]
}

/** `用户配置（衍生自 <source>）#n`, where n is the smallest free index for that source. */
export function defaultConfigName(baseLabel: string, existingNames: readonly string[]): string {
  const taken = new Set(existingNames)
  let index = 1
  while (taken.has(`用户配置（衍生自 ${baseLabel}）#${index}`)) index += 1
  return `用户配置（衍生自 ${baseLabel}）#${index}`
}

export type OptionCheck = { status: PresetStatus; warning: MessageKey | null }

/**
 * The display state of an encoding/locale combination. The backend owns `preset`
 * (factory combinations plus launch history); this only turns it into copy and
 * flags missing or unverified combinations without ever blocking a launch.
 */
export function optionsWarning(
  encoding: string | null,
  locale: string | null,
  preset: PresetStatus,
): OptionCheck {
  if (!encoding?.trim() || !locale?.trim())
    return { status: 'unverified', warning: 'optionPairMissing' }
  if (preset === 'failed') return { status: 'failed', warning: 'presetFailedHint' }
  if (preset === 'unverified') return { status: 'unverified', warning: 'presetUnverifiedHint' }
  return { status: 'verified', warning: null }
}

export type ChainLine = { key: MessageKey; value: string | null }

/** Thin config → factory config → effective ENCODING/LOCALE/SASUSER. */
export function chainLines(chain: Chain): ChainLine[] {
  return [
    { key: 'chainThinCfg', value: chain.thinCfg },
    { key: 'chainFactoryCfg', value: chain.factoryCfg },
    { key: 'chainEncoding', value: chain.encoding },
    { key: 'chainLocale', value: chain.locale },
    { key: 'chainSasuser', value: chain.sasuser },
    { key: 'chainLog', value: chain.log },
  ]
}

export function formatStartedAt(ms: number, language: Language): string {
  if (!Number.isFinite(ms) || ms <= 0) return '—'
  return new Intl.DateTimeFormat(language === 'en' ? 'en-US' : 'zh-CN', {
    year: 'numeric',
    month: '2-digit',
    day: '2-digit',
    hour: '2-digit',
    minute: '2-digit',
    hour12: false,
  }).format(new Date(ms))
}

/**
 * The region of a locale rendered as a language name (`zh_CN` → 简体中文).
 * Chinese needs the script form because `Intl` reports `中文（中国）` otherwise.
 */
/** SAS writes some factory `-LOCALE` values as bare English language names. */
const sasLanguageNames: Record<string, string> = {
  arabic: 'ar',
  danish: 'da',
  dutch: 'nl',
  french: 'fr',
  german: 'de',
  hebrew: 'he',
  hungarian: 'hu',
  italian: 'it',
  norwegian: 'no',
  polish: 'pl',
  portuguese: 'pt',
  russian: 'ru',
  spanish: 'es',
  swedish: 'sv',
  turkish: 'tr',
}

export function localeLanguageName(language: Language, locale: string | null): string | null {
  const value = locale?.trim()
  if (!value) return null
  const match = /^([A-Za-z]{2,3})(?:[-_]([A-Za-z]{2}))?$/.exec(value)
  if (!match) {
    return sasLanguageNames[value.toLowerCase()]
      ? localeLanguageName(language, sasLanguageNames[value.toLowerCase()])
      : value
  }
  const subtag = match[1].toLowerCase()
  const region = match[2]?.toUpperCase() ?? ''
  const code =
    subtag === 'zh' ? (['TW', 'HK', 'MO'].includes(region) ? 'zh-Hant' : 'zh-Hans') : subtag
  try {
    return (
      new Intl.DisplayNames([language], { type: 'language', fallback: 'none' }).of(code) ?? code
    )
  } catch {
    return code
  }
}

/** `简体中文 · EUC-CN`, used in the "follow the default configuration" hint. */ export function optionSummary(
  language: Language,
  config: Config | null,
): string {
  if (!config) return ''
  return [localeLanguageName(language, config.locale), config.encoding?.toUpperCase()]
    .filter((part): part is string => !!part)
    .join(' · ')
}

export function usageCount(projects: Project[], reference: ConfigRef): number {
  return projects.filter((project) => sameRef(project.config, reference)).length
}

export function configsByRef(configs: Config[], reference: ConfigRef | null): Config | undefined {
  if (!reference) return undefined
  return configs.find((config) => sameRef(config.reference, reference))
}

/** `{kind:'user', id:''}` is the "user configuration chosen but not picked yet" state. */
export function configReady(config: ConfigRef | null): boolean {
  if (!config) return true
  return config.kind === 'builtIn' ? config.code !== '' : config.id !== ''
}

export type PresetOption = { key: string; encoding: string; locale: string }

const presetKeyOf = (encoding: string, locale: string) =>
  encoding.trim().toLowerCase().replaceAll('-', '_') +
  '|' +
  locale.trim().toLowerCase().replaceAll('-', '_')

/** Unique (encoding, locale) combinations offered by the factory NLS configurations. */
export function factoryPresets(configs: Config[]): PresetOption[] {
  const seen = new Map<string, PresetOption>()
  for (const config of configs) {
    if (config.reference.kind !== 'builtIn' || !config.encoding || !config.locale) continue
    const key = presetKeyOf(config.encoding, config.locale)
    if (!seen.has(key)) seen.set(key, { key, encoding: config.encoding, locale: config.locale })
  }
  return [...seen.values()].sort(
    (a, b) => a.locale.localeCompare(b.locale) || a.encoding.localeCompare(b.encoding),
  )
}

/** The factory preset a pair of options belongs to, or `null` for a hand-written combination. */
export function matchPreset(
  presets: PresetOption[],
  encoding: string | null,
  locale: string | null,
): PresetOption | null {
  if (!encoding?.trim() || !locale?.trim()) return null
  const key = presetKeyOf(encoding, locale)
  return presets.find((preset) => preset.key === key) ?? null
}
