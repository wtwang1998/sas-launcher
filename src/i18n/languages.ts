import type { Language } from '../api'

/**
 * Everything that differs per interface language except the copy itself. One row per
 * language keeps the endonym, the date tag, the list separator and the collator from
 * drifting apart across five files, and `satisfies Record<Language, …>` makes a missing
 * row a type error rather than a fallback nobody notices.
 */
export type LanguageProfile = {
  /** The language named in its own words. A name, not copy, so it is not a message key. */
  endonym: string
  /** A BCP-47 tag `Intl.DateTimeFormat` accepts. */
  dateTag: string
  /**
   * What `joinList` puts between compact labels. CLDR's zh `Intl.ListFormat` unit pattern
   * has no separator at all and its conjunction pattern inserts 和/及, so the separator
   * stays explicit here instead of being inferred.
   */
  listSeparator: string
  /** A locale tag `Intl.Collator` accepts, used to order names the way this language does. */
  collatorLocale: string
}

export const languageProfiles = {
  'zh-Hans': {
    endonym: '简体中文',
    dateTag: 'zh-CN',
    listSeparator: '、',
    collatorLocale: 'zh-Hans',
  },
  'zh-Hant': {
    endonym: '繁體中文',
    dateTag: 'zh-TW',
    listSeparator: '、',
    collatorLocale: 'zh-Hant',
  },
  en: {
    endonym: 'English',
    dateTag: 'en-US',
    listSeparator: ', ',
    collatorLocale: 'en',
  },
  es: {
    endonym: 'Español',
    dateTag: 'es-ES',
    listSeparator: ', ',
    collatorLocale: 'es',
  },
  de: {
    endonym: 'Deutsch',
    dateTag: 'de-DE',
    listSeparator: ', ',
    collatorLocale: 'de',
  },
  ko: {
    endonym: '한국어',
    dateTag: 'ko-KR',
    listSeparator: ', ',
    collatorLocale: 'ko',
  },
  ja: {
    endonym: '日本語',
    dateTag: 'ja-JP',
    listSeparator: '、',
    collatorLocale: 'ja',
  },
} as const satisfies Record<Language, LanguageProfile>
