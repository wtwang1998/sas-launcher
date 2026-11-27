import { expect, it } from 'vitest'
import issues from '../issue-messages.json'
import baseline from './coverage-baseline.json'
import { messages as en, notes } from './en'
import { languageNames } from './index'
import { messages as zhHans } from './zh-Hans'
import { messages as zhHant } from './zh-Hant'

const catalogs = { 'zh-Hans': zhHans, 'zh-Hant': zhHant } as const

/** The `{name}` placeholders a message declares, independent of their order. */
function placeholders(value: string): string[] {
  return [...value.matchAll(/{(\w+)}/g)].map((match) => match[1]).sort()
}

it('a_catalog_only_names_keys_the_source_has', () => {
  const keys = new Set(Object.keys(en))
  for (const [language, catalog] of Object.entries(catalogs)) {
    for (const key of Object.keys(catalog)) {
      expect(keys.has(key), `${language} has an unknown key ${key}`).toBe(true)
    }
  }
})

it('a_translation_declares_the_same_placeholders_as_the_source', () => {
  for (const [language, catalog] of Object.entries(catalogs)) {
    for (const [key, value] of Object.entries(catalog)) {
      const source = en[key as keyof typeof en]
      expect(placeholders(value), `${language} ${key}`).toEqual(placeholders(source))
    }
  }
})

it('no_catalog_ships_an_empty_string', () => {
  for (const [, catalog] of Object.entries(catalogs)) {
    for (const [key, value] of Object.entries(catalog)) {
      expect(value.trim(), key).not.toBe('')
    }
  }
  for (const [key, value] of Object.entries(en)) {
    expect(value.trim(), key).not.toBe('')
  }
})

const sources = import.meta.glob(['/src/**/*.{ts,tsx}', '!**/*.test.{ts,tsx}', '!/src/i18n/**'], {
  query: '?raw',
  import: 'default',
  eager: true,
}) as Record<string, string>

it('every_source_key_is_used_somewhere', () => {
  const groups =
    'app|nav|settings|quit|notice|dialog|common|form|project|profile|configChoice|configurations|config|sessions|session|about'
  const pattern = new RegExp(`'(?:${groups})\\.[A-Za-z]+'`, 'g')
  const used = new Set<string>()
  for (const text of Object.values(sources)) {
    for (const match of text.matchAll(pattern)) used.add(match[0].slice(1, -1))
  }
  const unused = Object.keys(en).filter((key) => {
    if (used.has(key)) return false
    // A plural variant is reached through its base, which is the literal in the source.
    const base = key.replace(/\.(one|other)$/, '')
    return !(base !== key && used.has(base))
  })
  expect(unused, `unused message keys: ${unused.join(', ')}`).toEqual([])
})

it('no_language_drops_below_its_recorded_coverage', () => {
  const total = Object.keys(en).length
  for (const [language, floor] of Object.entries(baseline)) {
    const catalog = catalogs[language as keyof typeof catalogs]
    const missing = Object.keys(en).filter((key) => !(key in catalog))
    const translated = Object.keys(catalog).length
    expect(
      translated,
      `${language} lost coverage (${translated}/${total}); missing: ${missing.join(', ')}`,
    ).toBeGreaterThanOrEqual(floor)
  }
})

it('every_translator_note_names_a_key_the_source_has', () => {
  const keys = new Set(Object.keys(en))
  for (const key of Object.keys(notes)) {
    expect(keys.has(key as keyof typeof en), `note for unknown key ${key}`).toBe(true)
  }
})

it('names_each_interface_language_in_its_own_words', () => {
  for (const [language, name] of Object.entries(languageNames)) {
    const own = new Intl.DisplayNames([language], { type: 'language' }).of(language)
    expect(name, `${language} should read the way CLDR names it in itself`).toBe(own)
  }
})

it('writes every issue code in every interface language', () => {
  for (const [code, entry] of Object.entries(issues)) {
    for (const language of ['zh-Hans', 'zh-Hant', 'en'] as const) {
      expect(
        (entry as Partial<Record<string, string>>)[language],
        `${code} in ${language}`,
      ).toBeTruthy()
    }
  }
})
