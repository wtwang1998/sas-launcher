import { screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { expect, it } from 'vitest'
import type { BatchResult } from '../api'
import { builtIn, makeView } from '../test/fixtures'
import { renderApp } from '../test/harness'

it('submitting_default_following_project_sends_null_config', async () => {
  const user = userEvent.setup()
  const { backend } = renderApp(makeView({ configs: [builtIn('u8', 'UTF-8', 'zh_CN')] }), {
    sasuser_candidates: () => [],
  })

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
    profile: { kind: 'fresh', layers: { interface: true, environment: false, templates: false } },
  })
})

it('a_new_project_inherits_the_detected_user_folder_by_default', async () => {
  const user = userEvent.setup()
  const folder = 'C:\\Users\\me\\Documents\\My SAS Files\\9.4'
  const { backend } = renderApp(makeView({ configs: [builtIn('u8', 'UTF-8', 'zh_CN')] }), {
    sasuser_candidates: () => [{ path: folder, version: '9.4', files: 2, modified: 0 }],
  })

  await user.click(await screen.findByRole('button', { name: '新建项目' }))
  await user.type(screen.getByLabelText(/项目名称/), '项目 A')
  expect(await screen.findByText(/将继承 2 个文件/)).toBeInTheDocument()
  await user.click(screen.getByRole('button', { name: '保存项目' }))

  await waitFor(() =>
    expect(backend.calls.filter((call) => call.cmd === 'save_project')).toHaveLength(1),
  )
  expect(backend.calls.find((call) => call.cmd === 'save_project')?.args).toEqual({
    id: null,
    name: '项目 A',
    note: null,
    config: null,
    profile: {
      kind: 'sasUser',
      path: folder,
      layers: { interface: true, environment: false, templates: false },
    },
  })
})

it('a_missing_user_folder_still_creates_the_project_in_a_clean_environment', async () => {
  const user = userEvent.setup()
  const { backend } = renderApp(makeView({ configs: [builtIn('u8', 'UTF-8', 'zh_CN')] }), {
    sasuser_candidates: () => [],
  })

  await user.click(await screen.findByRole('button', { name: '新建项目' }))
  await user.type(screen.getByLabelText(/项目名称/), '项目 A')
  await user.click(screen.getByRole('button', { name: '保存项目' }))

  await waitFor(() =>
    expect(backend.calls.filter((call) => call.cmd === 'save_project')).toHaveLength(1),
  )
  const sent = backend.calls.find((call) => call.cmd === 'save_project')?.args?.profile
  expect((sent as { kind: string } | undefined)?.kind).toBe('fresh')
})

it('the_user_folder_group_is_named_without_the_sasuser_suffix', async () => {
  const user = userEvent.setup()
  const folder = 'C:\\Users\\me\\Documents\\My SAS Files\\9.4'
  renderApp(makeView({ configs: [builtIn('u8', 'UTF-8', 'zh_CN')] }), {
    sasuser_candidates: () => [
      { path: folder, version: '9.4', files: 4, modified: 1_700_000_000_000 },
    ],
  })

  await user.click(await screen.findByRole('button', { name: '新建项目' }))

  expect(await screen.findByText('用户偏好')).toBeInTheDocument()
  expect(screen.queryByText('用户偏好（SASUSER）')).not.toBeInTheDocument()
  expect(screen.getByLabelText('继承我的 SAS 用户目录')).toHaveDisplayValue(folder)
  expect(screen.queryByText(/来自/)).not.toBeInTheDocument()
  expect(await screen.findByText(/^将继承 4 个文件 · 修改于 /)).toBeInTheDocument()
})

it('a_disabled_snapshot_card_says_why_it_cannot_be_used', async () => {
  const user = userEvent.setup()
  renderApp(makeView({ configs: [builtIn('u8', 'UTF-8', 'zh_CN')] }), {
    sasuser_candidates: () => [],
  })

  await user.click(await screen.findByRole('button', { name: '新建项目' }))

  expect(await screen.findByText('没有已保存过会话设置的配置')).toBeInTheDocument()
})

it('the_layer_choices_belong_to_the_inheriting_card_only', async () => {
  const user = userEvent.setup()
  const folder = 'C:\\Users\\me\\Documents\\My SAS Files\\9.4'
  renderApp(makeView({ configs: [builtIn('u8', 'UTF-8', 'zh_CN')] }), {
    sasuser_candidates: () => [
      { path: folder, version: '9.4', files: 4, modified: 1_700_000_000_000 },
    ],
  })

  await user.click(await screen.findByRole('button', { name: '新建项目' }))

  // The interface layer cannot be turned off, so it is a sentence, not a tick.
  expect(await screen.findByText(/界面：/)).toBeInTheDocument()
  expect(screen.getByText(/始终包含/)).toBeInTheDocument()
  expect(screen.getByText('环境：库分配、Explorer 选项、打印设置')).toBeInTheDocument()

  await user.click(screen.getByRole('radio', { name: /干净环境/ }))

  expect(screen.queryByText('环境：库分配、Explorer 选项、打印设置')).not.toBeInTheDocument()
  expect(screen.queryByText(/始终包含/)).not.toBeInTheDocument()
})

it('name_is_required_and_trimmed', async () => {
  const user = userEvent.setup()
  const { backend } = renderApp(makeView({ configs: [builtIn('u8', 'UTF-8', 'zh_CN')] }), {
    sasuser_candidates: () => [],
  })

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
    profile: { kind: 'fresh', layers: { interface: true, environment: false, templates: false } },
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
