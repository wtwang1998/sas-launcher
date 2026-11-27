import { describe, expect, it } from 'vitest'
import type { Language } from '../api'
import { languageProfiles } from './languages'

const languages = Object.keys(languageProfiles).sort()

describe('language profiles', () => {
  it('covers exactly the interface languages', () => {
    expect(languages).toEqual(['de', 'en', 'es', 'ja', 'ko', 'zh-Hans', 'zh-Hant'])
  })

  it('names each language the way CLDR does', () => {
    for (const language of languages) {
      const endonym = new Intl.DisplayNames([language], { type: 'language' }).of(language) ?? ''
      expect(endonym, `CLDR has no name for ${language}`).not.toBe('')
      // CLDR lowercases some endonyms (Spanish spells itself espanol); a picker capitalizes it.
      expect(languageProfiles[language as Language].endonym.toLowerCase()).toBe(
        endonym.toLowerCase(),
      )
    }
  })

  it('keeps tags and separators the platform accepts', () => {
    for (const language of languages) {
      const profile = languageProfiles[language as Language]
      expect(() => new Intl.DateTimeFormat(profile.dateTag)).not.toThrow()
      expect(() => new Intl.Collator(profile.collatorLocale)).not.toThrow()
      expect(profile.listSeparator.length).toBeGreaterThan(0)
    }
  })
})
