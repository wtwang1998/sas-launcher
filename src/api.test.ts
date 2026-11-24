import { mockIPC } from '@tauri-apps/api/mocks'
import { beforeEach, expect, it } from 'vitest'
import { api } from './api'

type Call = { cmd: string; args?: Record<string, unknown> }
let calls: Call[] = []

beforeEach(() => {
  calls = []
  mockIPC((cmd, args) => {
    calls.push({ cmd, args: args as Record<string, unknown> | undefined })
  })
})

it('commands_match_the_rust_handler_names', async () => {
  const path = 'C:\\Program Files\\SASHome\\SASFoundation\\9.4\\sas.exe'
  const userRef = { kind: 'user', id: 'cfg-1' } as const
  const cases: {
    name: string
    run: () => Promise<unknown>
    cmd: string
    args: Record<string, unknown>
  }[] = [
    { name: 'snapshot', run: () => api.snapshot(), cmd: 'snapshot', args: {} },
    { name: 'refresh', run: () => api.refresh(), cmd: 'refresh_configs', args: {} },
    {
      name: 'selectInstallation',
      run: () => api.selectInstallation(path),
      cmd: 'select_installation',
      args: { path },
    },
    {
      name: 'setLanguage',
      run: () => api.setLanguage('en'),
      cmd: 'set_language',
      args: { preference: 'en' },
    },
    {
      name: 'clearInstallation',
      run: () => api.clearInstallation(),
      cmd: 'clear_installation',
      args: {},
    },
    {
      name: 'setTheme',
      run: () => api.setTheme('dark'),
      cmd: 'set_theme',
      args: { preference: 'dark' },
    },
    {
      name: 'setCloseToTray',
      run: () => api.setCloseToTray(false),
      cmd: 'set_close_to_tray',
      args: { enabled: false },
    },
    { name: 'hideToTray', run: () => api.hideToTray(), cmd: 'hide_to_tray', args: {} },
    { name: 'quit', run: () => api.quit(), cmd: 'quit', args: {} },
    {
      name: 'saveProject',
      run: () => api.saveProject('p1', '项目 A', '备注', userRef),
      cmd: 'save_project',
      args: { id: 'p1', name: '项目 A', note: '备注', config: userRef },
    },
    {
      name: 'saveProject with defaults',
      run: () => api.saveProject(null, '项目 B', null, null),
      cmd: 'save_project',
      args: { id: null, name: '项目 B', note: null, config: null },
    },
    {
      name: 'deleteProject',
      run: () => api.deleteProject('p1'),
      cmd: 'delete_project',
      args: { id: 'p1' },
    },
    {
      name: 'createConfiguration',
      run: () => api.createConfiguration({ kind: 'builtIn', code: 'u8' }, null),
      cmd: 'create_configuration',
      args: { base: { kind: 'builtIn', code: 'u8' }, name: null },
    },
    {
      name: 'updateConfiguration',
      run: () => api.updateConfiguration('cfg-1', 'UTF-8', 'zh_CN'),
      cmd: 'update_configuration',
      args: { id: 'cfg-1', encoding: 'UTF-8', locale: 'zh_CN' },
    },
    {
      name: 'deleteConfiguration',
      run: () => api.deleteConfiguration('cfg-1'),
      cmd: 'delete_configuration',
      args: { id: 'cfg-1' },
    },
    {
      name: 'startConfigSetup',
      run: () => api.startConfigSetup('cfg-1'),
      cmd: 'start_config_setup',
      args: { id: 'cfg-1' },
    },
    {
      name: 'finishConfigSetup',
      run: () => api.finishConfigSetup('cfg-1', true),
      cmd: 'finish_config_setup',
      args: { id: 'cfg-1', save: true },
    },
    {
      name: 'launchProject',
      run: () => api.launchProject('p1'),
      cmd: 'launch_project',
      args: { id: 'p1' },
    },
    {
      name: 'launchBatch',
      run: () => api.launchBatch(['p1', 'p2']),
      cmd: 'launch_batch',
      args: { ids: ['p1', 'p2'] },
    },
    {
      name: 'clearDiagnostics',
      run: () => api.clearDiagnostics(),
      cmd: 'clear_diagnostics',
      args: {},
    },
    { name: 'openLog', run: () => api.openLog('s1'), cmd: 'open_log', args: { id: 's1' } },
    {
      name: 'openSasuser',
      run: () => api.openSasuser('s1'),
      cmd: 'open_sasuser',
      args: { id: 's1' },
    },
    { name: 'openArchive', run: () => api.openArchive(), cmd: 'open_archive', args: {} },
  ]

  const wrappers = [
    'snapshot',
    'refresh',
    'selectInstallation',
    'setLanguage',
    'clearInstallation',
    'setTheme',
    'setCloseToTray',
    'hideToTray',
    'quit',
    'saveProject',
    'deleteProject',
    'createConfiguration',
    'updateConfiguration',
    'deleteConfiguration',
    'startConfigSetup',
    'finishConfigSetup',
    'launchProject',
    'launchBatch',
    'clearDiagnostics',
    'openLog',
    'openSasuser',
    'openArchive',
  ]
  expect(Object.keys(api).sort()).toEqual([...wrappers].sort())

  for (const entry of cases) {
    calls = []
    await entry.run()
    expect(calls, entry.name).toHaveLength(1)
    expect(calls[0].cmd, entry.name).toBe(entry.cmd)
    expect(calls[0].args ?? {}, entry.name).toEqual(entry.args)
    expect(Object.keys(calls[0].args ?? {}).sort(), entry.name).toEqual(
      Object.keys(entry.args).sort(),
    )
  }
})
