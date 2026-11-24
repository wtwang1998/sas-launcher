import { expect, it } from 'vitest'
import { startMinimizedActive, themeAttribute } from './settings'

it('only_an_explicit_choice_marks_the_document', () => {
  expect(themeAttribute('system')).toBeNull()
  expect(themeAttribute('dark')).toBe('dark')
  expect(themeAttribute('light')).toBe('light')
})

it('starting_minimized_needs_the_login_entry_to_be_registered', () => {
  expect(startMinimizedActive(true, true)).toBe(true)
  expect(startMinimizedActive(false, true)).toBe(false)
  expect(startMinimizedActive(true, false)).toBe(false)
  expect(startMinimizedActive(false, false)).toBe(false)
})
