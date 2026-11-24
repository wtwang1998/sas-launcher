import { expect, it } from 'vitest'
import { themeAttribute } from './settings'

it('only_an_explicit_choice_marks_the_document', () => {
  expect(themeAttribute('system')).toBeNull()
  expect(themeAttribute('dark')).toBe('dark')
  expect(themeAttribute('light')).toBe('light')
})
