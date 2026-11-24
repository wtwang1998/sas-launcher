import { screen, waitFor, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { expect, it } from 'vitest'
import { builtIn, makeSession, makeView, userConfig } from '../test/fixtures'
import { renderApp } from '../test/harness'

type User = ReturnType<typeof userEvent.setup>

async function openConfigurations(user: User) {
  const navigation = await screen.findByRole('navigation', { name: '主导航' })
  await user.click(within(navigation).getByRole('button', { name: '配置管理' }))
  await screen.findByRole('heading', { name: '配置管理' })
}

it('rows_show_name_left_and_chips_right', async () => {
  const user = userEvent.setup()
  renderApp(
    makeView({
      configs: [
        userConfig('cfg-1', '磐升瑞祥配置', 'EUC-CN', 'zh_CN'),
        builtIn('u8', 'UTF-8', 'zh_CN'),
      ],
    }),
  )
  await openConfigurations(user)

  const userRow = (await screen.findByText('磐升瑞祥配置')).closest('li')
  expect(userRow).not.toBeNull()
  expect(within(userRow as HTMLElement).getByText('简体中文')).toBeInTheDocument()
  expect(within(userRow as HTMLElement).getByText('EUC-CN')).toBeInTheDocument()
  expect(within(userRow as HTMLElement).getByText('衍生自 u8')).toBeInTheDocument()

  const builtInRow = screen.getByText('u8').closest('li')
  expect(builtInRow).not.toBeNull()
  expect(within(builtInRow as HTMLElement).getByText('简体中文')).toBeInTheDocument()
  expect(within(builtInRow as HTMLElement).getByText('UTF-8')).toBeInTheDocument()
})

it('built_in_rows_offer_only_derive', async () => {
  const user = userEvent.setup()
  renderApp(
    makeView({
      configs: [
        userConfig('cfg-1', '我的配置', 'EUC-CN', 'zh_CN'),
        builtIn('u8', 'UTF-8', 'zh_CN'),
      ],
    }),
  )
  await openConfigurations(user)

  const row = (await screen.findByText('u8')).closest('li')
  expect(row).not.toBeNull()
  await user.click(within(row as HTMLElement).getByRole('button', { name: /配置操作/ }))

  const items = within(row as HTMLElement).getAllByRole('menuitem')
  expect(items).toHaveLength(1)
  expect(items[0]).toHaveTextContent('从本配置衍生')
})

it('derive_sends_the_built_in_base', async () => {
  const user = userEvent.setup()
  const created = userConfig('cfg-1', '用户配置（衍生自 u8）#1', 'UTF-8', 'zh_CN')
  const { backend } = renderApp(makeView({ configs: [builtIn('u8', 'UTF-8', 'zh_CN')] }), {
    create_configuration: (_args, current) => {
      const next = { ...current.view, configs: [...current.view.configs, created] }
      current.setView(next)
      return next
    },
  })
  await openConfigurations(user)

  const row = (await screen.findByText('u8')).closest('li')
  expect(row).not.toBeNull()
  await user.click(within(row as HTMLElement).getByRole('button', { name: /配置操作/ }))
  await user.click(within(row as HTMLElement).getByRole('menuitem', { name: '从本配置衍生' }))

  await waitFor(() =>
    expect(backend.calls.filter((call) => call.cmd === 'create_configuration')).toHaveLength(1),
  )
  expect(backend.calls.find((call) => call.cmd === 'create_configuration')?.args).toEqual({
    base: { kind: 'builtIn', code: 'u8' },
    name: null,
  })
})

it('referenced_configuration_cannot_be_deleted', async () => {
  const user = userEvent.setup()
  renderApp(
    makeView({
      configs: [userConfig('cfg-1', '我的配置', 'EUC-CN', 'zh_CN')],
      projects: [
        {
          id: 'p1',
          name: '项目 A',
          note: null,
          config: { kind: 'user', id: 'cfg-1' },
          profile: null,
        },
      ],
    }),
  )
  await openConfigurations(user)

  const row = (await screen.findByText('我的配置')).closest('li')
  expect(row).not.toBeNull()
  expect(within(row as HTMLElement).getByText('被 1 个项目使用')).toBeInTheDocument()

  await user.click(within(row as HTMLElement).getByRole('button', { name: /配置操作/ }))
  expect(within(row as HTMLElement).getByRole('menuitem', { name: '删除配置' })).toBeDisabled()
})

it('setup_session_blocks_form_editing_and_offers_save_after_exit', async () => {
  const user = userEvent.setup()
  const config = userConfig('cfg-1', '我的配置', 'EUC-CN', 'zh_CN')

  const running = renderApp(
    makeView({
      configs: [config],
      sessions: [
        makeSession({
          id: 's1',
          kind: 'setup',
          configId: 'cfg-1',
          status: 'running',
          name: '设置会话',
        }),
      ],
    }),
  )
  await openConfigurations(user)

  const runningRow = (await screen.findByText('我的配置')).closest('li')
  expect(runningRow).not.toBeNull()
  expect(within(runningRow as HTMLElement).getByText('设置会话进行中')).toBeInTheDocument()
  await user.click(within(runningRow as HTMLElement).getByRole('button', { name: /配置操作/ }))
  expect(
    within(runningRow as HTMLElement).getByRole('menuitem', { name: '编辑设置' }),
  ).toBeDisabled()
  running.unmount()

  const { backend } = renderApp(
    makeView({
      configs: [config],
      sessions: [
        makeSession({
          id: 's1',
          kind: 'setup',
          configId: 'cfg-1',
          status: 'setup',
          exitCode: 0,
          name: '设置会话',
        }),
      ],
    }),
    {
      finish_config_setup: (args, current) => {
        if (args?.save) {
          const next = { ...current.view, sessions: [] }
          current.setView(next)
          return next
        }
        return current.view
      },
    },
  )
  await openConfigurations(user)

  const exitedRow = (await screen.findByText('我的配置')).closest('li')
  expect(exitedRow).not.toBeNull()
  await user.click(within(exitedRow as HTMLElement).getByRole('button', { name: '丢弃' }))
  await waitFor(() =>
    expect(backend.calls.filter((call) => call.cmd === 'finish_config_setup')).toHaveLength(1),
  )
  expect(backend.calls.filter((call) => call.cmd === 'finish_config_setup')[0].args).toEqual({
    id: 'cfg-1',
    save: false,
  })

  await user.click(within(exitedRow as HTMLElement).getByRole('button', { name: '保存这次的设置' }))
  await waitFor(() =>
    expect(backend.calls.filter((call) => call.cmd === 'finish_config_setup')).toHaveLength(2),
  )
  expect(backend.calls.filter((call) => call.cmd === 'finish_config_setup')[1].args).toEqual({
    id: 'cfg-1',
    save: true,
  })
})

it('editor_settings_limitation_is_stated_next_to_session_editing', async () => {
  const user = userEvent.setup()
  renderApp(makeView({ configs: [userConfig('cfg-1', '我的配置', 'EUC-CN', 'zh_CN')] }))
  await openConfigurations(user)

  const row = (await screen.findByText('我的配置')).closest('li')
  expect(row).not.toBeNull()
  await user.click(within(row as HTMLElement).getByRole('button', { name: /配置操作/ }))
  await user.click(within(row as HTMLElement).getByRole('menuitem', { name: '打开 SAS 调整设置' }))

  const dialog = await screen.findByRole('dialog')
  expect(
    within(dialog).getByText('SAS 的增强编辑器字体/颜色/按键存在 Windows 注册表，不随配置保存'),
  ).toBeInTheDocument()
})

it('edit_dialog_offers_factory_preset_combinations', async () => {
  const user = userEvent.setup()
  const { backend } = renderApp(
    makeView({
      configs: [
        userConfig('cfg-1', '我的配置', 'EUC-CN', 'zh_CN'),
        builtIn('u8', 'UTF-8', 'zh_CN'),
      ],
    }),
  )
  await openConfigurations(user)

  const row = (await screen.findByText('我的配置')).closest('li')
  expect(row).not.toBeNull()
  await user.click(within(row as HTMLElement).getByRole('button', { name: /配置操作/ }))
  await user.click(within(row as HTMLElement).getByRole('menuitem', { name: '编辑设置' }))

  const dialog = await screen.findByRole('dialog')
  const preset = within(dialog).getByLabelText('预设组合')
  await user.selectOptions(preset, within(preset).getByRole('option', { name: '简体中文 · UTF-8' }))
  expect(within(dialog).getByLabelText('编码')).toHaveValue('UTF-8')
  expect(within(dialog).getByLabelText('区域')).toHaveValue('zh_CN')

  await user.click(within(dialog).getByRole('button', { name: '保存' }))
  await waitFor(() =>
    expect(backend.calls.filter((call) => call.cmd === 'update_configuration')).toHaveLength(1),
  )
  expect(backend.calls.find((call) => call.cmd === 'update_configuration')?.args).toEqual({
    id: 'cfg-1',
    encoding: 'UTF-8',
    locale: 'zh_CN',
  })
})
