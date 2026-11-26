import { expect, it } from 'vitest'
import { formatArgv, formatModified, formatStartedAt, joinList, quoteArgument } from './format'

it('argv_is_quoted_like_a_windows_command_line', () => {
  const argv = [
    'C:\\SAS\\sas.exe',
    '-CONFIG',
    'C:\\Program Files\\SASHome\\nls\\u8\\sasv9.cfg',
    '-AWSTITLE',
    '项目 A · UTF-8',
    '-NORSASUSER',
  ]
  expect(quoteArgument('sas.exe')).toBe('sas.exe')
  expect(quoteArgument('C:\\SAS 9.4\\sasv9.cfg')).toBe('"C:\\SAS 9.4\\sasv9.cfg"')
  expect(formatArgv(argv)).toBe(
    'C:\\SAS\\sas.exe -CONFIG "C:\\Program Files\\SASHome\\nls\\u8\\sasv9.cfg" -AWSTITLE "项目 A · UTF-8" -NORSASUSER',
  )
  expect(formatArgv([])).toBe('')
})

it('list_join_uses_the_separator_of_the_language', () => {
  expect(joinList('zh-Hans', ['界面设置', '环境设置'])).toBe('界面设置、环境设置')
  expect(joinList('zh-Hans', ['a', 'b', 'c'])).toBe('a、b、c')
  expect(joinList('en', ['Interface', 'Environment'])).toBe('Interface, Environment')
  expect(joinList('en', ['a', 'b', 'c'])).toBe('a, b, c')
  expect(joinList('zh-Hans', [])).toBe('')
  expect(joinList('zh-Hans', ['only'])).toBe('only')
})

it('stamps_render_in_24_hour_form_and_keep_their_missing_value', () => {
  const stamp = Date.UTC(2026, 0, 2, 9, 5)
  expect(formatStartedAt(stamp, 'zh-Hans')).toContain('2026')
  expect(formatStartedAt(stamp, 'en')).toContain('2026')
  // 24-hour form in both languages, so a diagnostic never says AM/PM in one UI and 14:42 in the other.
  expect(formatStartedAt(stamp, 'en')).not.toMatch(/AM|PM/)
  expect(formatModified('en', stamp)).not.toMatch(/AM|PM/)
  expect(formatStartedAt(0, 'zh-Hans')).toBe('—')
  expect(formatStartedAt(Number.NaN, 'en')).toBe('—')
  expect(formatModified('zh-Hans', 0)).toBe('')
})
