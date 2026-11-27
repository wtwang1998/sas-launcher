import type { Language } from '../api'
import issues from '../issue-messages.json'
import { messages as en, type MessageKey } from './en'
import { messages as de } from './de'
import { messages as ko } from './ko'
import { messages as ja } from './ja'
import { messages as zhHans } from './zh-Hans'
import { messages as zhHant } from './zh-Hant'
import { languageProfiles } from './languages'

/** One catalog per language. English is the source, so the other languages are partial. */
const catalogs: Record<Language, Partial<Record<MessageKey, string>>> = {
  en,
  de: de,
  ko: ko,
  ja,
  'zh-Hans': zhHans,
  'zh-Hant': zhHant,
}

export type { MessageKey }

export type Translator = (key: MessageKey, values?: Record<string, string | number>) => string

/**
 * Each interface language named in its own words, the way CLDR names it. These are names,
 * not copy, so they live in the language profile instead of the catalogs.
 */
export const languageNames = Object.fromEntries(
  Object.entries(languageProfiles).map(([language, profile]) => [language, profile.endonym]),
) as Record<Language, string>

/** English is the source: an untranslated key reads in English, never blank. */
function text(language: Language, key: MessageKey): string {
  return catalogs[language][key] ?? en[key]
}

function interpolate(template: string, values: Record<string, string | number>): string {
  return template.replace(/{(\w+)}/g, (_, name: string) => String(values[name] ?? ''))
}

/**
 * One key in one language. `translate` below and every hook built on it come through
 * here, so a table-driven test can drive the same seam the UI uses.
 */
/**
 * The dev-only pseudo locale: every message renders as its key, so a label that is still
 * English, or one that outgrows its button, is visible without reading the catalogs.
 */
export function pseudoText(key: MessageKey, values: Record<string, string | number> = {}): string {
  const args = Object.entries(values)
    .map(([name, value]) => `${name}=${value}`)
    .join(', ')
  return args ? `[[${key} | ${args}]]` : `[[${key}]]`
}

/** `?pseudo=1` in a development build; never reachable in a packaged one. */
function pseudoEnabled(): boolean {
  const env = (import.meta as { env?: { DEV?: boolean } }).env
  if (!env?.DEV || typeof window === 'undefined') return false
  return new URLSearchParams(window.location.search).has('pseudo')
}

export function translateKey(
  language: Language,
  key: MessageKey,
  values: Record<string, string | number> = {},
): string {
  if (pseudoEnabled()) return pseudoText(key, values)
  return interpolate(text(language, key), values)
}

export function translate(language: Language): Translator {
  return (key, values) => translateKey(language, key, values)
}

/** The messages that carry a singular and a plural variant. */
export type PluralBase =
  | 'quit.quitMessage'
  | 'config.usedByProjects'
  | 'profile.inheritUserFolderSummary'
  | 'session.profileIncomplete'
  | 'session.profileSeedValue'

/**
 * The variant a count selects: CLDR says English has one and other, Chinese only other.
 * The choice comes from `Intl.PluralRules`, never from `count === 1`, so a language whose
 * rules differ needs no code change.
 */
export function pluralKey(language: Language, base: PluralBase, count: number): MessageKey {
  const category = new Intl.PluralRules(language).select(count) === 'one' ? 'one' : 'other'
  return `${base}.${category}` as MessageKey
}

export function issueText(language: Language, error: unknown) {
  const catalog = issues as unknown as Record<string, Partial<Record<Language, string>>>
  const known =
    error && typeof error === 'object' && 'code' in error && typeof error.code === 'string'
      ? catalog[error.code]
      : undefined
  const detail =
    error && typeof error === 'object' && 'detail' in error && typeof error.detail === 'string'
      ? error.detail
      : typeof error === 'string'
        ? error
        : error instanceof Error
          ? error.message
          : ''
  // A code that has no entry for this language yet reads in English, never blank.
  const entry = known ?? catalog.unexpected
  const title = entry[language] ?? entry.en ?? ''
  return detail ? `${title} (${detail})` : title
}

/**
 * The language to use before the first snapshot arrives. Windows reports the display
 * language as a BCP-47 tag, so the script subtag decides Chinese; everything else is English.
 */
export function browserLanguage(): Language {
  const tag = navigator.language.toLowerCase()
  if (tag.startsWith('zh')) {
    if (/(?:^|-)tw(?:-|$)|(?:^|-)hk(?:-|$)|(?:^|-)mo(?:-|$)|hant/.test(tag)) return 'zh-Hant'
    return 'zh-Hans'
  }
  return 'en'
}
