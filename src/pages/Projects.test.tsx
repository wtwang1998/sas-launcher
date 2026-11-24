import { screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { expect, it } from 'vitest'
import type { BatchResult } from '../api'
import { builtIn, makeView } from '../test/fixtures'
import { renderApp } from '../test/harness'

it('submitting_default_following_project_sends_null_config', async () => {
  const user = userEvent.setup()
  const { backend } = renderApp(makeView({ configs: [builtIn('u8', 'UTF-8', 'zh_CN')] }))

  await user.click(await screen.findByRole('button', { name: '新建项目' }))
  await user.type(screen.getByLabelText(/项目名称/), '项目 A')
  await user.click(screen.getByRole('button', { name: '保存项目' }))

  await waitFor(() =>
    expect(backend.calls.filter((call) => call.cmd === 'save_project')).toHaveLength(1),
  )
  expect(backend.calls.find((call) => call.cmd === 'save_project')?.args).toEqual({
    id: null,
    name: '项目 A',
    note: null,
    config: null,
    profile: null,
  })
})

it('name_is_required_and_trimmed', async () => {
  const user = userEvent.setup()
  const { backend } = renderApp(makeView({ configs: [builtIn('u8', 'UTF-8', 'zh_CN')] }))

  await user.click(await screen.findByRole('button', { name: '新建项目' }))
  const name = screen.getByLabelText(/项目名称/)
  await user.type(name, '   ')
  await user.click(screen.getByRole('button', { name: '保存项目' }))

  expect(screen.getByText('请填写项目名称')).toBeInTheDocument()
  expect(backend.calls.some((call) => call.cmd === 'save_project')).toBe(false)

  await user.clear(name)
  await user.type(name, '  项目 A  ')
  await user.click(screen.getByRole('button', { name: '保存项目' }))

  await waitFor(() =>
    expect(backend.calls.filter((call) => call.cmd === 'save_project')).toHaveLength(1),
  )
  expect(backend.calls.find((call) => call.cmd === 'save_project')?.args).toEqual({
    id: null,
    name: '项目 A',
    note: null,
    config: null,
    profile: null,
  })
})

it('batch_launch_sends_selected_ids', async () => {
  const user = userEvent.setup()
  const view = makeView({
    configs: [builtIn('u8', 'UTF-8', 'zh_CN')],
    projects: [
      { id: 'p1', name: '项目 A', note: null, config: null, profile: null },
      {
        id: 'p2',
        name: '项目 B',
        note: null,
        config: { kind: 'builtIn', code: 'u8' },
        profile: null,
      },
    ],
  })
  const { backend } = renderApp(view, { launch_batch: () => [] as BatchResult[] })

  await user.click(await screen.findByRole('checkbox', { name: '选择项目 项目 A' }))
  await user.click(screen.getByRole('checkbox', { name: '选择项目 项目 B' }))
  await user.click(screen.getByRole('button', { name: /启动所选/ }))

  await waitFor(() =>
    expect(backend.calls.filter((call) => call.cmd === 'launch_batch')).toHaveLength(1),
  )
  expect(backend.calls.find((call) => call.cmd === 'launch_batch')?.args).toEqual({
    ids: ['p1', 'p2'],
  })
})
