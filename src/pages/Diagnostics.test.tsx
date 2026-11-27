import { act, screen, waitFor, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { expect, it, vi } from 'vitest'
import type { Chain, SessionLog } from '../api'
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

const errorLog: SessionLog = {
  state: 'ok',
  errors: 1,
  warnings: 1,
  entries: [
    {
      line: 6643,
      severity: 'error',
      text: 'ERROR: 文件正在使用中，E:\\统计部\\项目\\MD\\2024\\05\\setup.sas',
    },
    {
      line: 6648,
      severity: 'warning',
      text: 'WARNING: 没有 Body（主体）文件。不会创建 RTF 输出。',
    },
  ],
  truncated: false,
  scannedBytes: 1_536,
}

async function openSession(name: string) {
  const navigation = await screen.findByRole('navigation', { name: '主导航' })
  await userEvent.setup().click(within(navigation).getByRole('button', { name: '会话诊断' }))
  await userEvent.setup().click(await screen.findByRole('button', { name: `展开 ${name} 的详情` }))
}

it('an_expanded_session_offers_its_counts_as_two_chips', async () => {
  renderApp(
    makeView({ sessions: [makeSession({ id: 's1', name: '项目 A', errors: 7, warnings: 6 })] }),
  )
  await openSession('项目 A')

  expect(screen.getByRole('button', { name: 'ERROR 7' })).toBeInTheDocument()
  expect(screen.getByRole('button', { name: 'WARNING 6' })).toBeInTheDocument()
})

it('a_session_with_nothing_to_report_offers_no_chips', async () => {
  renderApp(makeView({ sessions: [makeSession({ id: 's1', name: '项目 A' })] }))
  await openSession('项目 A')

  expect(screen.queryByRole('button', { name: /^ERROR / })).not.toBeInTheDocument()
  expect(screen.queryByRole('button', { name: /^WARNING / })).not.toBeInTheDocument()
})

it('the_error_chip_lists_the_entries_and_focuses_the_first_one', async () => {
  const { backend } = renderApp(
    makeView({ sessions: [makeSession({ id: 's1', name: '项目 A', errors: 1, warnings: 1 })] }),
    { session_log: () => errorLog },
  )
  await openSession('项目 A')
  await userEvent.setup().click(screen.getByRole('button', { name: 'ERROR 1' }))

  expect(await screen.findByText(/文件正在使用中/)).toBeInTheDocument()
  expect(backend.calls.filter((call) => call.cmd === 'session_log')).toHaveLength(1)
  const row = (await screen.findByText(/文件正在使用中/)).closest('li')
  expect(row).toHaveTextContent('6643')
  expect(document.activeElement).toBe(row)
})

it('the_warning_chip_focuses_the_first_warning_instead', async () => {
  renderApp(
    makeView({ sessions: [makeSession({ id: 's1', name: '项目 A', errors: 1, warnings: 1 })] }),
    { session_log: () => errorLog },
  )
  await openSession('项目 A')
  await userEvent.setup().click(screen.getByRole('button', { name: 'WARNING 1' }))

  const row = (await screen.findByText(/没有 Body/)).closest('li')
  expect(row).toHaveTextContent('6648')
  expect(document.activeElement).toBe(row)
})

it('a_session_without_a_log_says_so_instead_of_zero', async () => {
  renderApp(
    makeView({
      sessions: [
        makeSession({
          id: 's1',
          name: '项目 A',
          chain: { ...makeSession({ id: 's0' }).chain, log: null },
        }),
      ],
    }),
  )
  await openSession('项目 A')

  expect(screen.getByText('这个会话还没有日志。')).toBeInTheDocument()
  expect(screen.queryByRole('button', { name: /^ERROR / })).not.toBeInTheDocument()
})

it('a_truncated_report_says_the_counts_are_a_floor', async () => {
  renderApp(
    makeView({ sessions: [makeSession({ id: 's1', name: '项目 A', errors: 1, warnings: 1 })] }),
    { session_log: () => ({ ...errorLog, truncated: true, scannedBytes: 8 * 1024 * 1024 }) },
  )
  await openSession('项目 A')
  await userEvent.setup().click(screen.getByRole('button', { name: 'ERROR 1' }))

  expect(await screen.findByText(/仅前 2000 条/)).toBeInTheDocument()
  // The heading was dropped: the size is not information worth a line.
  expect(screen.queryByText(/已读/)).not.toBeInTheDocument()
})

it('pressing_the_open_chip_closes_the_list_and_returns_focus', async () => {
  const { backend } = renderApp(
    makeView({ sessions: [makeSession({ id: 's1', name: '项目 A', errors: 1, warnings: 1 })] }),
    { session_log: () => errorLog },
  )
  await openSession('项目 A')
  const user = userEvent.setup()
  const chip = screen.getByRole('button', { name: 'ERROR 1' })

  await user.click(chip)
  expect(await screen.findByText(/文件正在使用中/)).toBeInTheDocument()

  await user.click(chip)
  expect(screen.queryByText(/文件正在使用中/)).not.toBeInTheDocument()
  expect(chip).toHaveAttribute('aria-pressed', 'false')
  expect(document.activeElement).toBe(chip)

  // Reopening scans again: the list is a snapshot of "now", not a cache.
  await user.click(chip)
  expect(await screen.findByText(/文件正在使用中/)).toBeInTheDocument()
  expect(backend.calls.filter((call) => call.cmd === 'session_log')).toHaveLength(2)
})

it('collapsing_the_session_card_forgets_the_open_list', async () => {
  renderApp(
    makeView({ sessions: [makeSession({ id: 's1', name: '项目 A', errors: 1, warnings: 1 })] }),
    { session_log: () => errorLog },
  )
  await openSession('项目 A')
  const user = userEvent.setup()
  await user.click(screen.getByRole('button', { name: 'ERROR 1' }))
  expect(await screen.findByText(/文件正在使用中/)).toBeInTheDocument()

  const toggle = screen.getByRole('button', { name: '展开 项目 A 的详情' })
  await user.click(toggle)
  await user.click(toggle)

  expect(screen.queryByText(/文件正在使用中/)).not.toBeInTheDocument()
  expect(screen.getByRole('button', { name: 'ERROR 1' })).toHaveAttribute('aria-pressed', 'false')
})
it('copying_a_line_says_so_on_that_button', async () => {
  renderApp(
    makeView({ sessions: [makeSession({ id: 's1', name: '项目 A', errors: 1, warnings: 1 })] }),
    { session_log: () => errorLog },
  )
  await openSession('项目 A')
  const user = userEvent.setup()
  await user.click(screen.getByRole('button', { name: 'ERROR 1' }))

  await user.click(await screen.findByRole('button', { name: '复制第 6643 行' }))

  expect(await navigator.clipboard.readText()).toBe(
    'sas.log:6643 ERROR: 文件正在使用中，E:\\统计部\\项目\\MD\\2024\\05\\setup.sas',
  )
  expect(await screen.findByRole('button', { name: '已复制第 6643 行' })).toBeInTheDocument()
})

it('a_copy_the_browser_refuses_says_so_on_the_same_button', async () => {
  renderApp(
    makeView({ sessions: [makeSession({ id: 's1', name: '项目 A', errors: 1, warnings: 1 })] }),
    { session_log: () => errorLog },
  )
  await openSession('项目 A')
  const user = userEvent.setup()
  await user.click(screen.getByRole('button', { name: 'ERROR 1' }))
  const copy = await screen.findByRole('button', { name: '复制第 6643 行' })
  vi.spyOn(navigator.clipboard, 'writeText').mockRejectedValueOnce(new Error('denied'))

  await user.click(copy)

  expect(await screen.findByRole('button', { name: '复制第 6643 行失败' })).toBeInTheDocument()
})

it('the_copy_confirmation_goes_away_after_two_seconds', async () => {
  renderApp(
    makeView({ sessions: [makeSession({ id: 's1', name: '项目 A', errors: 1, warnings: 1 })] }),
    { session_log: () => errorLog },
  )
  await openSession('项目 A')
  await userEvent.setup().click(screen.getByRole('button', { name: 'ERROR 1' }))
  const copy = await screen.findByRole('button', { name: '复制第 6643 行' })

  // fake timers 从点击这一刻开始：user-event 自己的等待靠真定时器，交给 act 之外。
  vi.useFakeTimers()
  try {
    await act(async () => {
      copy.click()
    })
    expect(screen.getByRole('button', { name: '已复制第 6643 行' })).toBeInTheDocument()

    act(() => {
      vi.advanceTimersByTime(1999)
    })
    expect(screen.getByRole('button', { name: '已复制第 6643 行' })).toBeInTheDocument()

    act(() => {
      vi.advanceTimersByTime(1)
    })
    expect(screen.getByRole('button', { name: '复制第 6643 行' })).toBeInTheDocument()
  } finally {
    vi.useRealTimers()
  }
})
