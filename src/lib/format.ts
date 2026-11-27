import type { Language } from '../api'
import { languageProfiles } from '../i18n/languages'

/** Quote one OS argument the way Windows shows it in a command line. */
export function quoteArgument(value: string): string {
  return /[\s"]/.test(value) ? `"${value.replaceAll('"', '\\"')}"` : value
}

/** Format the recorded argv as a readable, copyable command line. */
export function formatArgv(argv: readonly string[]): string {
  return argv.map(quoteArgument).join(' ')
}

/**
 * CLDR gives the zh `Intl.ListFormat` unit pattern no separator at all ("ABC"), and its
 * conjunction pattern inserts 和/及. These labels are compact (`界面设置、环境设置`), so the
 * separator stays explicit per language, in `languageProfiles`.
 */

/** Join a compact list of already translated labels. */
export function joinList(language: Language, items: readonly string[]): string {
  return items.join(languageProfiles[language].listSeparator)
}

/** `2026/01/02 17:05`; a missing or out-of-range stamp renders as `—`. */
export function formatStartedAt(millis: number, language: Language): string {
  if (!Number.isFinite(millis) || millis <= 0) return '—'
  return dateTime(language, millis)
}

/** The form's "last changed": an absent stamp renders as nothing at all. */
export function formatModified(language: Language, millis: number): string {
  if (!Number.isFinite(millis) || millis <= 0) return ''
  return dateTime(language, millis)
}

/** One rendering for every timestamp: 24-hour in every language, never AM/PM. */
function dateTime(language: Language, millis: number): string {
  return new Intl.DateTimeFormat(languageProfiles[language].dateTag, {
    year: 'numeric',
    month: '2-digit',
    day: '2-digit',
    hour: '2-digit',
    minute: '2-digit',
    hour12: false,
  }).format(new Date(millis))
}
