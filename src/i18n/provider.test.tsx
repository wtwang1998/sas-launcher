import { render, screen } from '@testing-library/react'
import { expect, it } from 'vitest'
import { I18nProvider, useT } from './provider'

/** A control that expects a provider above it. */
function Orphan() {
  useT()
  return null
}

/** A control that really translates, to read the language the provider hands down. */
function ProjectsLabel() {
  return <span>{useT()('nav.projects')}</span>
}

it('a_control_outside_the_provider_fails_loudly', () => {
  // Rendering a control with no provider must break, not silently fall back to Chinese.
  expect(() => render(<Orphan />)).toThrow(/useI18n/)
})

it('the_provider_hands_out_the_language_it_was_given', () => {
  render(
    <I18nProvider language="zh-Hant">
      <ProjectsLabel />
    </I18nProvider>,
  )

  expect(screen.getByText('專案')).toBeInTheDocument()
})
