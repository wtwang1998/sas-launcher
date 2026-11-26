import { screen, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { expect, it } from 'vitest'
import { makeView } from '../test/fixtures'
import { renderApp } from '../test/harness'

async function openAbout(user: ReturnType<typeof userEvent.setup>) {
  const navigation = await screen.findByRole('navigation', { name: '主导航' })
  await user.click(within(navigation).getByRole('button', { name: '关于' }))
}

it('the_about_page_names_the_version_and_the_commit_it_was_built_from', async () => {
  const user = userEvent.setup()
  renderApp(makeView({ version: '9.9.9', commit: 'abc1234-dirty' }))

  await openAbout(user)

  expect(await screen.findByRole('heading', { name: '关于' })).toBeInTheDocument()
  expect(screen.getByText('v9.9.9')).toBeInTheDocument()
  expect(screen.getByText('abc1234-dirty')).toBeInTheDocument()
  expect(screen.getByText('GPL-3.0-or-later')).toBeInTheDocument()
})

it('copying_the_version_writes_one_pasteable_line', async () => {
  const user = userEvent.setup()
  renderApp(makeView({ version: '9.9.9', commit: 'abc1234' }))

  await openAbout(user)
  await user.click(await screen.findByRole('button', { name: '复制版本信息' }))

  expect(await navigator.clipboard.readText()).toBe('SAS Launcher 9.9.9 (abc1234)')
  expect(screen.getByRole('button', { name: '已复制' })).toBeInTheDocument()
})

it('a_link_that_will_not_open_says_so_where_it_was_clicked', async () => {
  const user = userEvent.setup()
  const { backend } = renderApp(makeView(), {
    open_issues: () => Promise.reject({ code: 'openLinkFailed', detail: 'explorer.exe' }),
  })

  await openAbout(user)
  await user.click(await screen.findByRole('button', { name: '报告问题' }))

  expect(backend.calls.map((call) => call.cmd)).toContain('open_issues')
  expect(await screen.findByText(/无法在浏览器中打开链接/)).toBeInTheDocument()
  expect(screen.getByRole('button', { name: '报告问题' })).toBeEnabled()
})
