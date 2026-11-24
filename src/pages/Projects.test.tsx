import { screen, waitFor, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { expect, it, vi } from 'vitest'
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
    profile: { kind: 'fresh', layers: { interface: true, environment: true, templates: false } },
  })
})

it('a_new_project_inherits_the_detected_user_folder_by_default', async () => {
  const user = userEvent.setup()
  const folder = 'C:\\Users\\me\\Documents\\My SAS Files\\9.4'
  const { backend } = renderApp(makeView({ configs: [builtIn('u8', 'UTF-8', 'zh_CN')] }), {
    sasuser_candidates: () => [
      {
        path: folder,
        version: '9.4',
        files: 2,
        present: { interface: 2, environment: 0, templates: 0 },
        modified: 0,
      },
    ],
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
      layers: { interface: true, environment: true, templates: false },
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
      {
        path: folder,
        version: '9.4',
        files: 4,
        present: { interface: 2, environment: 1, templates: 1 },
        modified: 1_700_000_000_000,
      },
    ],
  })

  await user.click(await screen.findByRole('button', { name: '新建项目' }))

  expect(await screen.findByText('用户偏好')).toBeInTheDocument()
  expect(screen.queryByText('用户偏好（SASUSER）')).not.toBeInTheDocument()
  expect(screen.getByLabelText('继承我的 SAS 用户目录')).toHaveDisplayValue(folder)
  expect(screen.queryByText(/来自/)).not.toBeInTheDocument()
  // Four catalogs exist, but the ODS templates stay behind: the count must
  // promise what the snapshot will really hold, not what the folder holds.
  expect(
    await screen.findByText(/^将继承 3 个文件 · 修改于 /),
    'the copy names the files the chosen layers carry',
  ).toBeInTheDocument()
})

it('a_disabled_snapshot_card_says_why_it_cannot_be_used', async () => {
  const user = userEvent.setup()
  renderApp(makeView({ configs: [builtIn('u8', 'UTF-8', 'zh_CN')] }), {
    sasuser_candidates: () => [],
  })

  await user.click(await screen.findByRole('button', { name: '新建项目' }))

  expect(await screen.findByText('没有已保存过会话设置的配置')).toBeInTheDocument()
})

it('a_user_folder_card_without_candidates_offers_browsing_inside_the_card', async () => {
  const user = userEvent.setup()
  renderApp(makeView({ configs: [builtIn('u8', 'UTF-8', 'zh_CN')] }), {
    sasuser_candidates: () => [],
  })

  await user.click(await screen.findByRole('button', { name: '新建项目' }))

  // With nothing detected the card stays selectable, because browsing can still
  // supply the folder; the link then lives inside the card that owns the path.
  const card = await screen.findByRole('radio', { name: /继承我的 SAS 用户目录/ })
  expect(card).not.toHaveAttribute('aria-disabled', 'true')

  await user.click(card)

  expect(within(card).getByText('未找到 My SAS Files 下的用户偏好目录')).toBeInTheDocument()
  expect(within(card).getByRole('button', { name: '浏览其他文件夹…' })).toBeInTheDocument()
})

it('the_layer_choices_belong_to_the_inheriting_card_only', async () => {
  const user = userEvent.setup()
  const folder = 'C:\\Users\\me\\Documents\\My SAS Files\\9.4'
  renderApp(makeView({ configs: [builtIn('u8', 'UTF-8', 'zh_CN')] }), {
    sasuser_candidates: () => [
      {
        path: folder,
        version: '9.4',
        files: 4,
        present: { interface: 2, environment: 1, templates: 1 },
        modified: 1_700_000_000_000,
      },
    ],
  })

  await user.click(await screen.findByRole('button', { name: '新建项目' }))

  // The interface layer cannot be turned off, so it is a sentence, not a tick:
  // exactly the two optional layers have a tick box.
  expect(
    await screen.findByText(/窗口布局、功能键、菜单与资源管理器视图会沿用/),
  ).toBeInTheDocument()
  expect(screen.getByText('还可以带上：')).toBeInTheDocument()
  expect(
    screen.getByRole('checkbox', { name: '环境（库分配、Explorer 选项、打印设置）' }),
  ).toBeInTheDocument()
  expect(screen.getByRole('checkbox', { name: 'ODS 模板与样式' })).toBeInTheDocument()
  expect(screen.getAllByRole('checkbox')).toHaveLength(2)

  await user.click(screen.getByRole('radio', { name: /不使用已有偏好/ }))

  expect(screen.queryByText(/窗口布局、功能键、菜单与资源管理器视图会沿用/)).not.toBeInTheDocument()
  expect(screen.queryByRole('checkbox', { name: /环境/ })).not.toBeInTheDocument()
  expect(screen.queryByRole('checkbox', { name: 'ODS 模板与样式' })).not.toBeInTheDocument()
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
    profile: { kind: 'fresh', layers: { interface: true, environment: true, templates: false } },
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

it('a_failed_save_explains_itself_next_to_the_buttons', async () => {
  const user = userEvent.setup()
  renderApp(makeView({ configs: [builtIn('u8', 'UTF-8', 'zh_CN')] }), {
    sasuser_candidates: () => [],
    save_project: () => {
      throw { code: 'unexpected', detail: 'profile busy' }
    },
  })

  await user.click(await screen.findByRole('button', { name: '新建项目' }))
  await user.type(screen.getByLabelText(/项目名称/), '项目 A')
  await user.click(screen.getByRole('button', { name: '保存项目' }))

  const alert = await screen.findByRole('alert')
  expect(alert).toHaveTextContent('profile busy')
  // It belongs to the form, not to a banner above the page.
  expect(alert.closest('.edit-panel')).not.toBeNull()
  expect(document.querySelector('.content > .banner')).toBeNull()
})

it('a_failed_launch_explains_itself_in_the_project_list', async () => {
  const user = userEvent.setup()
  const project = {
    id: 'p1',
    name: '项目 A',
    note: null,
    config: null,
    profile: null,
  }
  renderApp(makeView({ projects: [project] }), {
    launch_project: () => {
      throw { code: 'unexpected', detail: 'exe missing' }
    },
  })

  await user.click(await screen.findByRole('button', { name: '启动' }))

  const alert = await screen.findByRole('alert')
  expect(alert).toHaveTextContent('exe missing')
  expect(alert.classList.contains('page-notice')).toBe(true)
})

it('a_failed_delete_keeps_the_confirmation_open_and_says_why', async () => {
  const user = userEvent.setup()
  const project = {
    id: 'p1',
    name: '项目 A',
    note: null,
    config: null,
    profile: null,
  }
  renderApp(makeView({ projects: [project] }), {
    delete_project: () => {
      throw { code: 'unexpected', detail: 'still running' }
    },
  })

  await user.click(await screen.findByRole('button', { name: '删除 项目 A' }))
  const dialog = await screen.findByRole('dialog')
  await user.click(within(dialog).getByRole('button', { name: '删除' }))

  const alert = await within(dialog).findByRole('alert')
  expect(alert).toHaveTextContent('still running')
  expect(screen.getByRole('dialog')).toBeInTheDocument()
})

it('a_notice_brings_itself_into_view', async () => {
  const user = userEvent.setup()
  const scrolled = vi.fn()
  Element.prototype.scrollIntoView = scrolled
  renderApp(makeView({ configs: [builtIn('u8', 'UTF-8', 'zh_CN')] }), {
    sasuser_candidates: () => [],
    save_project: () => {
      throw { code: 'unexpected', detail: 'still busy' }
    },
  })

  await user.click(await screen.findByRole('button', { name: '新建项目' }))
  await user.type(screen.getByLabelText(/项目名称/), '项目 A')
  await user.click(screen.getByRole('button', { name: '保存项目' }))

  await screen.findByRole('alert')
  expect(scrolled).toHaveBeenCalled()
})

it('a_saved_project_reports_on_the_page_once_the_form_closes', async () => {
  const user = userEvent.setup()
  renderApp(makeView({ configs: [builtIn('u8', 'UTF-8', 'zh_CN')] }), {
    sasuser_candidates: () => [],
  })

  await user.click(await screen.findByRole('button', { name: '新建项目' }))
  await user.type(screen.getByLabelText(/项目名称/), '项目 A')
  await user.click(screen.getByRole('button', { name: '保存项目' }))

  const status = await screen.findByRole('status')
  expect(status).toHaveTextContent('项目已保存')
  expect(status.classList.contains('page-notice')).toBe(true)
  expect(screen.queryByLabelText(/项目名称/)).not.toBeInTheDocument()
})
