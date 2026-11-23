import { expect, it } from 'vitest'
import { formatArgv, quoteArgument } from './format'

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
