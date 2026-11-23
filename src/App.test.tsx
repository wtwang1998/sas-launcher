import { screen, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { expect, it } from 'vitest'
import { builtIn, makeView, userConfig } from './test/fixtures'
import { renderApp } from './test/harness'

it('exactly_three_navigation_items_without_an_install_path_entry', async () => {
  renderApp(makeView({ configs: [builtIn('u8', 'UTF-8', 'zh_CN')] }))

  const navigation = await screen.findByRole('navigation', { name: '主导航' })
  const items = within(navigation).getAllByRole('button')
  expect(items.map((item) => item.textContent)).toEqual(['项目', '配置管理', '会话诊断'])
  expect(screen.queryByText('管理 SAS 程序路径')).not.toBeInTheDocument()
})

it('scan_error_shows_a_banner_with_the_exe_picker', async () => {
  renderApp(makeView({ exe: null, scanError: { code: 'sasMissing' } }))

  const message = await screen.findByText(/未找到 SAS 9.4/)
  const banner = message.closest('.banner')
  expect(banner).not.toBeNull()
  expect(
    within(banner as HTMLElement).getByRole('button', { name: '选择 sas.exe' }),
  ).toBeInTheDocument()
})

it('draft_survives_a_trip_to_configuration_management', async () => {
  const user = userEvent.setup()
  const created = userConfig('cfg-1', '用户配置（衍生自 u8）#1', 'UTF-8', 'zh_CN')
  const { backend } = renderApp(makeView({ configs: [builtIn('u8', 'UTF-8', 'zh_CN')] }), {
    create_configuration: (_args, current) => {
      const next = { ...current.view, configs: [...current.view.configs, created] }
      current.setView(next)
      return next
    },
  })

  await screen.findByRole('heading', { name: '项目' })
  await user.click(screen.getByRole('button', { name: '新建项目' }))
  await user.type(screen.getByLabelText(/项目名称/), '项目 A')
  await user.click(screen.getByRole('button', { name: '创建新配置' }))

  expect(await screen.findByRole('heading', { name: '配置管理' })).toBeInTheDocument()
  expect(screen.getByText('还没有用户配置')).toBeInTheDocument()

  const builtInRow = screen.getByText('u8').closest('li')
  expect(builtInRow).not.toBeNull()
  await user.click(within(builtInRow as HTMLElement).getByRole('button', { name: /配置操作/ }))
  await user.click(
    within(builtInRow as HTMLElement).getByRole('menuitem', { name: '从本配置衍生' }),
  )

  expect(backend.calls.find((call) => call.cmd === 'create_configuration')?.args).toMatchObject({
    base: { kind: 'builtIn', code: 'u8' },
    name: null,
  })
  expect(await screen.findByText('用户配置（衍生自 u8）#1')).toBeInTheDocument()

  await user.click(await screen.findByRole('button', { name: /返回未完成的新增项目/ }))

  expect(await screen.findByLabelText(/项目名称/)).toHaveValue('项目 A')
  const userOption = screen.getByRole('radio', { name: /用户配置/ })
  expect(userOption).toHaveAttribute('aria-checked', 'true')
  expect(within(userOption).getByRole('combobox')).toHaveTextContent('用户配置（衍生自 u8）#1')
})
