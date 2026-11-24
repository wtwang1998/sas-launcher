import { screen, waitFor, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { expect, it } from 'vitest'
import type { Chain } from '../api'
import { makeSession, makeView } from '../test/fixtures'
import { renderApp } from '../test/harness'

it('row_shows_stage_and_exit_code', async () => {
  const user = userEvent.setup()
  renderApp(
    makeView({
      sessions: [
        makeSession({ id: 's1', name: '项目 A', status: 'exited', exitCode: 5, stage: 'spawn' }),
      ],
    }),
  )

  const navigation = await screen.findByRole('navigation', { name: '主导航' })
  await user.click(within(navigation).getByRole('button', { name: '会话诊断' }))

  expect(await screen.findByText('项目 A')).toBeInTheDocument()
  expect(screen.getByText('已退出')).toBeInTheDocument()
  expect(screen.getByText('失败阶段：创建进程')).toBeInTheDocument()
  expect(screen.getByText('退出码 5')).toBeInTheDocument()
})

it('chain_lists_thin_factory_and_effective_options', async () => {
  const user = userEvent.setup()
  const chain: Chain = {
    label: '用户配置（衍生自 u8）#1',
    thinCfg: 'C:\\data\\configurations\\cfg-1\\sasv9.cfg',
    factoryCfg: 'C:\\SASHome\\SASFoundation\\9.4\\nls\\u8\\sasv9.cfg',
    encoding: 'UTF-8',
    locale: 'zh_CN',
    profile: null,
    sasuser: 'C:\\data\\sessions\\s1\\sasuser',
    log: 'C:\\data\\sessions\\s1\\SAS.LOG',
  }
  renderApp(
    makeView({
      sessions: [
        makeSession({ id: 's1', name: '项目 A', status: 'running', stage: 'started', chain }),
      ],
    }),
  )

  const navigation = await screen.findByRole('navigation', { name: '主导航' })
  await user.click(within(navigation).getByRole('button', { name: '会话诊断' }))
  await user.click(await screen.findByRole('button', { name: '展开 项目 A 的详情' }))

  expect(screen.getByText('覆盖配置')).toBeInTheDocument()
  expect(screen.getByText(chain.thinCfg!)).toBeInTheDocument()
  expect(screen.getByText('基线配置')).toBeInTheDocument()
  expect(screen.getByText(chain.factoryCfg!)).toBeInTheDocument()
  expect(screen.getByText('UTF-8')).toBeInTheDocument()
  expect(screen.getByText('zh_CN')).toBeInTheDocument()
  expect(screen.getByText(chain.sasuser)).toBeInTheDocument()
})

it('clear_removes_records_only_after_confirmation', async () => {
  const user = userEvent.setup()
  const { backend } = renderApp(
    makeView({
      sessions: [makeSession({ id: 's1', name: '项目 A', status: 'exited', exitCode: 0 })],
    }),
    {
      clear_diagnostics: (_args, current) => {
        const next = { ...current.view, sessions: [] }
        current.setView(next)
        return next
      },
    },
  )

  const navigation = await screen.findByRole('navigation', { name: '主导航' })
  await user.click(within(navigation).getByRole('button', { name: '会话诊断' }))
  await user.click(await screen.findByRole('button', { name: '清空记录' }))

  const dialog = await screen.findByRole('dialog')
  expect(backend.calls.some((call) => call.cmd === 'clear_diagnostics')).toBe(false)

  await user.click(within(dialog).getByRole('button', { name: '清空记录' }))
  await waitFor(() =>
    expect(backend.calls.filter((call) => call.cmd === 'clear_diagnostics')).toHaveLength(1),
  )
  expect(await screen.findByText('还没有会话记录')).toBeInTheDocument()
})

it('preflight_hints_are_listed_for_an_expanded_session', async () => {
  const user = userEvent.setup()
  renderApp(
    makeView({
      sessions: [
        makeSession({
          id: 's1',
          name: '项目 A',
          status: 'exited',
          exitCode: 1,
          stage: 'spawn',
          hints: [{ code: 'configUnavailable', detail: 'C:\\SASHome\\nls\\u8\\sasv9.cfg' }],
        }),
      ],
    }),
  )

  const navigation = await screen.findByRole('navigation', { name: '主导航' })
  await user.click(within(navigation).getByRole('button', { name: '会话诊断' }))
  await user.click(await screen.findByRole('button', { name: '展开 项目 A 的详情' }))

  expect(screen.getByText('自检提示')).toBeInTheDocument()
  expect(screen.getByText(/C:\\SASHome\\nls\\u8\\sasv9\.cfg/)).toBeInTheDocument()
})

it('a_failed_launch_shows_its_stage_and_reason', async () => {
  const user = userEvent.setup()
  renderApp(
    makeView({
      sessions: [
        makeSession({
          id: 's1',
          name: '项目 A',
          status: 'exited',
          exitCode: null,
          stage: 'cfg',
          failure: { code: 'configurationFileMissing' },
        }),
      ],
    }),
  )

  const navigation = await screen.findByRole('navigation', { name: '主导航' })
  await user.click(within(navigation).getByRole('button', { name: '会话诊断' }))
  expect(await screen.findByText('失败阶段：读取配置')).toBeInTheDocument()

  await user.click(screen.getByRole('button', { name: '展开 项目 A 的详情' }))
  expect(screen.getByText(/启动配置文件不可用/)).toBeInTheDocument()
})
