import { expect, it } from 'vitest'
import { copyVersion } from './about'

it('the_copied_line_pastes_into_an_issue_as_is', () => {
  const cases = [
    { version: '0.3.0', commit: '1feed56', expected: 'SAS Launcher 0.3.0 (1feed56)' },
    {
      version: '0.3.0',
      commit: '1feed56-dirty',
      expected: 'SAS Launcher 0.3.0 (1feed56-dirty)',
    },
    { version: '1.10.0', commit: 'unknown', expected: 'SAS Launcher 1.10.0 (unknown)' },
  ]

  for (const { version, commit, expected } of cases) {
    expect(copyVersion(version, commit)).toBe(expected)
  }
})
