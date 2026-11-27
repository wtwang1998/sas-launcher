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
  expect(screen.queryByText('已找到 SAS 程序')).not.toBeInTheDocument()
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

it('traditional_chinese_is_offered_and_sent_to_the_backend', async () => {
  const user = userEvent.setup()
  const { backend } = renderApp(makeView())

  await user.click(await screen.findByRole('button', { name: '设置' }))
  const languagePicker = screen.getByRole('combobox', { name: '应用语言' })
  expect(within(languagePicker).getByRole('option', { name: '繁體中文' })).toBeInTheDocument()

  await user.selectOptions(languagePicker, 'zh-Hant')

  expect(backend.calls.find((call) => call.cmd === 'set_language')?.args).toMatchObject({
    preference: 'zh-Hant',
  })
})

it('a_traditional_interface_renders_its_own_copy', async () => {
  renderApp(makeView({ language: 'zh-Hant' }))

  expect(await screen.findByRole('heading', { name: '專案' })).toBeInTheDocument()
  expect(screen.getByRole('navigation', { name: '主要導覽' })).toBeInTheDocument()
})

it('an_english_interface_renders_its_own_copy', async () => {
  const user = userEvent.setup()
  renderApp(makeView({ language: 'en', languagePreference: 'en' }))

  const navigation = await screen.findByRole('navigation', { name: 'Main navigation' })
  await user.click(within(navigation).getByRole('button', { name: 'Settings' }))

  expect(await screen.findByRole('heading', { name: 'Settings' })).toBeInTheDocument()
  expect(screen.getByRole('combobox', { name: 'App language' })).toHaveValue('en')
  expect(screen.getByRole('textbox', { name: 'SAS executable path' })).toHaveValue(
    'C:\\SAS\\sas.exe',
  )
})

it('following_windows_shows_which_language_it_resolves_to', async () => {
  const user = userEvent.setup()
  renderApp(makeView({ languagePreference: 'system', systemLanguage: 'zh-TW' }))

  await user.click(await screen.findByRole('button', { name: '设置' }))
  expect(screen.getByText('跟随系统（当前：繁体中文）')).toBeInTheDocument()
})

it('an_explicit_language_hides_the_resolved_system_language', async () => {
  const user = userEvent.setup()
  renderApp(makeView({ languagePreference: 'zh-Hans', systemLanguage: 'zh-TW' }))

  await user.click(await screen.findByRole('button', { name: '设置' }))
  expect(screen.queryByText(/跟随系统（当前/)).not.toBeInTheDocument()
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

it('the_close_to_tray_switch_reflects_and_updates_the_setting', async () => {
  const user = userEvent.setup()
  const { backend } = renderApp(makeView({ closeToTray: true }))

  await user.click(await screen.findByRole('button', { name: '设置' }))
  const toggle = screen.getByRole('switch', { name: '关闭时最小化到托盘' })
  expect(toggle).toHaveAttribute('aria-checked', 'true')

  await user.click(toggle)

  expect(backend.calls.find((call) => call.cmd === 'set_close_to_tray')?.args).toMatchObject({
    enabled: false,
  })
})

it('the_login_switch_is_persisted_through_set_startup', async () => {
  const user = userEvent.setup()
  const { backend } = renderApp(makeView({ autostart: false }))

  await user.click(await screen.findByRole('button', { name: '设置' }))
  const toggle = screen.getByRole('switch', { name: '开机时自动启动' })
  expect(toggle).toHaveAttribute('aria-checked', 'false')
  await user.click(toggle)

  expect(backend.calls.find((call) => call.cmd === 'set_startup')?.args).toMatchObject({
    enabled: true,
  })
})

it('starting_minimized_only_exists_once_the_login_entry_is_on', async () => {
  const user = userEvent.setup()
  renderApp(makeView({ autostart: false, startMinimized: true }))

  await user.click(await screen.findByRole('button', { name: '设置' }))

  expect(screen.queryByRole('switch', { name: '启动时最小化到托盘' })).not.toBeInTheDocument()
  expect(screen.queryByText('需先开启「开机时自动启动」')).not.toBeInTheDocument()
})

it('starting_minimized_is_editable_once_the_login_entry_exists', async () => {
  const user = userEvent.setup()
  const { backend } = renderApp(makeView({ autostart: true, startMinimized: false }))

  await user.click(await screen.findByRole('button', { name: '设置' }))
  const toggle = screen.getByRole('switch', { name: '启动时最小化到托盘' })
  expect(toggle).toBeEnabled()
  await user.click(toggle)

  expect(backend.calls.find((call) => call.cmd === 'set_start_minimized')?.args).toMatchObject({
    enabled: true,
  })
})

it('a_failed_switch_reports_inside_its_own_group', async () => {
  const user = userEvent.setup()
  renderApp(makeView({ closeToTray: true }), {
    set_close_to_tray: () => {
      throw { code: 'unexpected', detail: 'registry locked' }
    },
  })

  await user.click(await screen.findByRole('button', { name: '设置' }))
  await user.click(screen.getByRole('switch', { name: '关闭时最小化到托盘' }))

  const alert = await screen.findByRole('alert')
  expect(alert).toHaveTextContent('registry locked')
  expect(alert.closest('.settings-group')).not.toBeNull()
  expect(alert.closest('.panel')?.textContent).toContain('关闭时最小化到托盘')
  expect(document.querySelector('.content > .banner')).toBeNull()
})

it('names_each_language_in_its_own_words', async () => {
  const cases = [
    {
      language: 'zh-Hans',
      navigation: '主导航',
      settings: '设置',
      picker: '应用语言',
      system: '跟随系统',
    },
    {
      language: 'zh-Hant',
      navigation: '主要導覽',
      settings: '設定',
      picker: '應用程式語言',
      system: '跟隨系統',
    },
    {
      language: 'en',
      navigation: 'Main navigation',
      settings: 'Settings',
      picker: 'App language',
      system: 'Follow System',
    },
    {
      language: 'ja',
      navigation: 'メインナビゲーション',
      settings: '設定',
      picker: '表示言語',
      system: 'システムに従う',
    },
    {
      language: 'ko',
      navigation: '기본 탐색',
      settings: '설정',
      picker: '앱 언어',
      system: '시스템 따르기',
    },
    {
      language: 'de',
      navigation: 'Hauptnavigation',
      settings: 'Einstellungen',
      picker: 'App-Sprache',
      system: 'Systemeinstellung folgen',
    },
  ] as const

  for (const one of cases) {
    const user = userEvent.setup()
    const { unmount } = renderApp(
      makeView({ language: one.language, languagePreference: one.language }),
    )

    const navigation = await screen.findByRole('navigation', { name: one.navigation })
    await user.click(within(navigation).getByRole('button', { name: one.settings }))

    const picker = await screen.findByRole('combobox', { name: one.picker })
    const labels = within(picker)
      .getAllByRole('option')
      .map((option) => option.textContent)
    expect(labels, `language options in a ${one.language} interface`).toEqual([
      one.system,
      '简体中文',
      '繁體中文',
      'English',
      'Deutsch',
      '日本語',
      '한국어',
    ])
    const traditional = within(picker).getByRole('option', { name: '繁體中文' })
    expect(traditional.getAttribute('lang'), 'a name should carry its own language').toBe('zh-Hant')
    const deName = within(picker).getByRole('option', { name: 'Deutsch' })
    expect(deName.getAttribute('lang'), 'a name should carry its own language').toBe('de')
    const koName = within(picker).getByRole('option', { name: '한국어' })
    expect(koName.getAttribute('lang'), 'a name should carry its own language').toBe('ko')
    const japanese = within(picker).getByRole('option', { name: '日本語' })
    expect(japanese.getAttribute('lang'), 'a name should carry its own language').toBe('ja')

    unmount()
  }
})
