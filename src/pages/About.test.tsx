import { screen, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, expect, it, vi } from 'vitest'
import { makeView } from '../test/fixtures'
import { renderApp } from '../test/harness'

afterEach(() => {
  vi.unstubAllGlobals()
})

/** A releases response, as the page would receive it from GitHub. */
const releases = (body: unknown) => {
  return async () =>
    ({
      ok: true,
      status: 200,
      headers: new Headers(),
      json: async () => body,
    }) as unknown as Response
}

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

it('a_check_names_a_newer_release_and_opens_its_page', async () => {
  const user = userEvent.setup()
  vi.stubGlobal('fetch', releases([{ tag_name: 'v0.4.0', draft: false, prerelease: false }]))
  const { backend } = renderApp(makeView({ version: '0.3.0' }))

  await openAbout(user)
  await user.click(await screen.findByRole('button', { name: '检查更新' }))

  expect(await screen.findByText('有新版本 v0.4.0。')).toBeInTheDocument()
  await user.click(screen.getByRole('button', { name: '打开发布页' }))
  expect(backend.calls.find((call) => call.cmd === 'open_release')?.args).toEqual({
    tag: 'v0.4.0',
  })
})

it('a_pre_release_is_named_as_one', async () => {
  const user = userEvent.setup()
  vi.stubGlobal('fetch', releases([{ tag_name: 'v0.5.0-rc.1', draft: false, prerelease: true }]))
  renderApp(makeView({ version: '0.3.0' }))

  await openAbout(user)
  await user.click(await screen.findByRole('button', { name: '检查更新' }))

  expect(await screen.findByText('有预发布版 v0.5.0-rc.1。')).toBeInTheDocument()
})

it('a_check_that_finds_nothing_new_says_so', async () => {
  const user = userEvent.setup()
  vi.stubGlobal('fetch', releases([{ tag_name: 'v0.3.0', draft: false, prerelease: true }]))
  renderApp(makeView({ version: '0.3.0' }))

  await openAbout(user)
  await user.click(await screen.findByRole('button', { name: '检查更新' }))

  expect(await screen.findByText('已是最新版本（v0.3.0）。')).toBeInTheDocument()
  expect(screen.getByRole('button', { name: '检查更新' })).toBeEnabled()
})

it('a_check_that_cannot_reach_github_says_why_and_keeps_the_button', async () => {
  const user = userEvent.setup()
  vi.stubGlobal('fetch', async () => {
    throw new TypeError('Failed to fetch')
  })
  renderApp(makeView({ version: '0.3.0' }))

  await openAbout(user)
  await user.click(await screen.findByRole('button', { name: '检查更新' }))

  expect(await screen.findByText('无法连接 GitHub，请检查网络后重试。')).toBeInTheDocument()
  expect(screen.getByRole('button', { name: '检查更新' })).toBeEnabled()
})
