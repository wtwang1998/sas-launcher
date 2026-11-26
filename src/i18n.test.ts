import { expect, it } from 'vitest'
import { translate } from './i18n'
import { messages as en } from './i18n/en'
import { messages as zhHans } from './i18n/zh-Hans'

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
  expect(translate('zh-Hans')('config.usedByProjects', { count: 1 })).toBe('被 1 个项目使用')
  expect(translate('en')('session.exitCodeValue', { code: 5 })).toBe('Exit code 5')
})
