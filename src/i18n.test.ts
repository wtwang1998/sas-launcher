import { expect, it } from 'vitest'
import { pseudoText, translate } from './i18n'
import { messages as en } from './i18n/en'
import { messages as zhHans } from './i18n/zh-Hans'
import { messages as zhHant } from './i18n/zh-Hant'

it('english_is_the_source_and_every_chinese_key_is_known', () => {
  const keys = Object.keys(en)
  expect(keys.length).toBeGreaterThan(200)
  for (const key of keys) {
    expect(en[key as keyof typeof en].trim(), `${key} (en)`).not.toBe('')
  }

  for (const [key, value] of Object.entries(zhHans)) {
    expect(keys.includes(key), `unknown key ${key}`).toBe(true)
    expect(value.trim(), `${key} (zh-Hans)`).not.toBe('')
  }
})

it('placeholders_are_interpolated_in_both_languages', () => {
  expect(translate('zh-Hans')('config.derivedFrom', { name: 'u8' })).toBe('衍生自 u8')
  expect(translate('en')('config.derivedFrom', { name: 'u8' })).toBe('Derived from u8')
  expect(translate('zh-Hans')('config.usedByProjects.other', { count: 1 })).toBe('被 1 个项目使用')
  expect(translate('en')('session.exitCodeValue', { code: 5 })).toBe('Exit code 5')
})

it('traditional_chinese_knows_every_key_and_renders_its_own_copy', () => {
  const keys = Object.keys(en)
  expect(Object.keys(zhHant)).toHaveLength(keys.length)
  for (const [key, value] of Object.entries(zhHant)) {
    expect(keys.includes(key), `unknown key ${key}`).toBe(true)
    expect(value.trim(), `${key} (zh-Hant)`).not.toBe('')
  }
  expect(translate('zh-Hant')('nav.projects')).toBe('專案')
  expect(translate('zh-Hant')('session.exitCodeValue', { code: 5 })).toBe('結束碼 5')
})

it('the_pseudo_locale_shows_the_key_instead_of_the_copy', () => {
  expect(pseudoText('nav.projects')).toBe('[[nav.projects]]')
  expect(pseudoText('session.exitCodeValue', { code: 5 })).toBe(
    '[[session.exitCodeValue | code=5]]',
  )
})
