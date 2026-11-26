import { expect, it } from 'vitest'
import { messages, translate } from './i18n'
import issues from './issue-messages.json'

it('every_message_has_both_languages', () => {
  for (const [key, value] of Object.entries(messages)) {
    expect(Array.isArray(value), key).toBe(true)
    expect(value, key).toHaveLength(2)
    expect(typeof value[0], key).toBe('string')
    expect(typeof value[1], key).toBe('string')
    expect(value[0].trim(), `${key} (zh-Hans)`).not.toBe('')
    expect(value[1].trim(), `${key} (en)`).not.toBe('')
  }

  for (const [code, value] of Object.entries(issues)) {
    expect(value, code).toHaveLength(2)
    expect(typeof value[0], code).toBe('string')
    expect(value[0].trim(), `${code} (zh-Hans)`).not.toBe('')
    expect(value[1].trim(), `${code} (en)`).not.toBe('')
  }
})

it('placeholders_are_interpolated_in_both_languages', () => {
  expect(translate('zh-Hans')('config.derivedFrom', { name: 'u8' })).toBe('衍生自 u8')
  expect(translate('en')('config.derivedFrom', { name: 'u8' })).toBe('Derived from u8')
  expect(translate('zh-Hans')('config.usedByProjects', { count: 1 })).toBe('被 1 个项目使用')
  expect(translate('en')('session.exitCodeValue', { code: 5 })).toBe('Exit code 5')
})
