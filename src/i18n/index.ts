import type { Language } from '../api'
import issues from '../issue-messages.json'
import { messages as en, type MessageKey } from './en'
import { messages as zhHans } from './zh-Hans'
import { messages as zhHant } from './zh-Hant'

export type { MessageKey }

export type Translator = (key: MessageKey, values?: Record<string, string | number>) => string

/** One catalog per language. English is the source, so the other languages are partial. */
const catalogs: Record<Language, Partial<Record<MessageKey, string>>> = {
  en,
  'zh-Hans': zhHans,
  'zh-Hant': zhHant,
}

/** The index of a language inside a `[zh-Hans, en, zh-Hant]` catalog entry. */
const issueIndex: Record<Language, number> = { 'zh-Hans': 0, en: 1, 'zh-Hant': 2 }

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
export function translateKey(
  language: Language,
  key: MessageKey,
  values: Record<string, string | number> = {},
): string {
  return interpolate(text(language, key), values)
}

export function translate(language: Language): Translator {
  return (key, values) => translateKey(language, key, values)
}

export function issueText(language: Language, error: unknown) {
  const catalog = issues as unknown as Record<string, [string, string, string?]>
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
  const title = entry[issueIndex[language]] ?? entry[1]
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
