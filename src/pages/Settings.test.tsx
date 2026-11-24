import { screen, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { expect, it } from 'vitest'
import { builtIn, makeView } from '../test/fixtures'
import { renderApp } from '../test/harness'

it('settings_page_owns_the_language_switch_and_the_sas_path', async () => {
  const user = userEvent.setup()
  renderApp(makeView({ configs: [builtIn('u8', 'UTF-8', 'zh_CN')] }))

  const navigation = await screen.findByRole('navigation', { name: '主导航' })
  expect(within(navigation).queryByRole('combobox')).not.toBeInTheDocument()
  expect(screen.queryByText('已找到 SAS 程序')).not.toBeInTheDocument()

  await user.click(within(navigation).getByRole('button', { name: '设置' }))

  expect(await screen.findByRole('heading', { name: '设置' })).toBeInTheDocument()
  expect(screen.getByRole('combobox', { name: '应用语言' })).toHaveValue('zh-Hans')
  expect(screen.getByRole('textbox', { name: 'SAS 程序路径' })).toHaveValue('C:\\SAS\\sas.exe')
  expect(screen.getByText('已找到 SAS 程序')).toBeInTheDocument()
})

it('changing_the_language_from_settings_calls_set_language', async () => {
  const user = userEvent.setup()
  const { backend } = renderApp(makeView())

  await user.click(await screen.findByRole('button', { name: '设置' }))
  await user.selectOptions(screen.getByRole('combobox', { name: '应用语言' }), 'en')

  expect(backend.calls.find((call) => call.cmd === 'set_language')?.args).toMatchObject({
    preference: 'en',
  })
})

it('saving_an_edited_path_pins_the_installation', async () => {
  const user = userEvent.setup()
  const { backend } = renderApp(makeView(), {
    select_installation: (args, current) => {
      const next = { ...current.view, exe: String(args?.path) }
      current.setView(next)
      return next
    },
  })

  await user.click(await screen.findByRole('button', { name: '设置' }))
  const field = screen.getByRole('textbox', { name: 'SAS 程序路径' })
  await user.clear(field)
  await user.type(field, 'D:\\SASHome\\sas.exe')
  await user.click(screen.getByRole('button', { name: '保存路径' }))

  expect(backend.calls.find((call) => call.cmd === 'select_installation')?.args).toMatchObject({
    path: 'D:\\SASHome\\sas.exe',
  })
})

it('auto_detect_drops_a_manual_override', async () => {
  const user = userEvent.setup()
  const { backend } = renderApp(makeView())

  await user.click(await screen.findByRole('button', { name: '设置' }))
  await user.click(screen.getByRole('button', { name: '自动检测' }))

  expect(backend.calls.some((call) => call.cmd === 'clear_installation')).toBe(true)
})

it('the_theme_choice_is_persisted_through_set_theme', async () => {
  const user = userEvent.setup()
  const { backend } = renderApp(makeView({ themePreference: 'system' }))

  await user.click(await screen.findByRole('button', { name: '设置' }))
  const theme = screen.getByRole('combobox', { name: '主题' })
  expect(theme).toHaveValue('system')
  await user.selectOptions(theme, 'dark')

  expect(backend.calls.find((call) => call.cmd === 'set_theme')?.args).toMatchObject({
    preference: 'dark',
  })
})

it('an_explicit_theme_marks_the_document_and_following_system_does_not', async () => {
  const explicit = renderApp(makeView({ themePreference: 'dark' }))
  await screen.findByRole('heading', { name: '项目' })
  expect(document.documentElement.dataset.theme).toBe('dark')
  explicit.unmount()

  renderApp(makeView({ themePreference: 'system' }))
  await screen.findByRole('heading', { name: '项目' })
  expect(document.documentElement.dataset.theme).toBeUndefined()
})
