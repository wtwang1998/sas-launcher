import { screen, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { expect, it } from 'vitest'
import { makeSession, makeView } from './test/fixtures'
import { renderApp } from './test/harness'

/**
 * The shell decides what a window close means (`close_intent` in main.rs); it
 * only asks the frontend when leaving would strand running SAS sessions.
 */
const hostEvent = (name: string) => document.dispatchEvent(new Event(name))

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
