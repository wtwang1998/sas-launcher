import { screen, waitFor, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { expect, it } from 'vitest'
import { makeSession, makeView } from './test/fixtures'
import { renderApp } from './test/harness'

/** The Rust shell emits these; the shell test dispatches the same names. */
const hostEvent = (name: string) => document.dispatchEvent(new Event(name))

it('the_first_trip_to_the_tray_says_where_the_window_went', async () => {
  const user = userEvent.setup()
  const { backend } = renderApp(makeView({ closeToTray: true, trayHintSeen: false }))
  await screen.findByRole('heading', { name: '项目' })

  hostEvent('close-requested')

  const dialog = await screen.findByRole('dialog')
  expect(within(dialog).getByText(/窗口将隐藏到系统托盘/)).toBeInTheDocument()
  await user.click(within(dialog).getByRole('button', { name: '最小化到托盘' }))

  expect(backend.calls.some((call) => call.cmd === 'hide_to_tray')).toBe(true)
})

it('once_the_hint_was_read_closing_hides_without_a_dialog', async () => {
  const { backend } = renderApp(makeView({ closeToTray: true, trayHintSeen: true }))
  await screen.findByRole('heading', { name: '项目' })

  hostEvent('close-requested')

  await waitFor(() => expect(backend.calls.some((call) => call.cmd === 'hide_to_tray')).toBe(true))
  expect(screen.queryByRole('dialog')).not.toBeInTheDocument()
})

it('quitting_while_sas_is_running_asks_before_it_exits', async () => {
  const user = userEvent.setup()
  const { backend } = renderApp(makeView({ sessions: [makeSession({ id: 's1' })] }))
  await screen.findByRole('heading', { name: '项目' })

  hostEvent('quit-requested')

  const dialog = await screen.findByRole('dialog')
  expect(within(dialog).getByText(/1 个 SAS 会话/)).toBeInTheDocument()
  await user.click(within(dialog).getByRole('button', { name: '退出' }))

  expect(backend.calls.some((call) => call.cmd === 'quit')).toBe(true)
})

it('cancelling_the_quit_keeps_the_launcher_open', async () => {
  const user = userEvent.setup()
  const { backend } = renderApp(makeView({ sessions: [makeSession({ id: 's1' })] }))
  await screen.findByRole('heading', { name: '项目' })

  hostEvent('quit-requested')
  const dialog = await screen.findByRole('dialog')
  await user.click(within(dialog).getByRole('button', { name: '取消' }))

  expect(backend.calls.some((call) => call.cmd === 'quit')).toBe(false)
})
