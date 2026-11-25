import type { Language } from './api'
import issues from './issue-messages.json'

export const messages = {
  title: ['SAS 启动管理器', 'SAS Launcher'],
  navigation: ['主导航', 'Main navigation'],
  projects: ['项目', 'Projects'],
  configurations: ['配置管理', 'Configurations'],
  sessions: ['会话诊断', 'Session Diagnostics'],
  projectsHelp: [
    '保存常用项目的配置，一键启动 SAS。',
    'Save a configuration for each project and start SAS in one click.',
  ],
  configurationsHelp: [
    '维护用户配置的编码与区域；内置配置只读。',
    'Adjust the encoding and locale of your own configurations. Built-in configurations are read-only.',
  ],
  sessionsHelp: [
    '查看会话状态、解析链与真实启动参数。',
    'Review session status, resolution chains and real launch arguments.',
  ],

  language: ['应用语言', 'App language'],
  languageZhHans: ['简体中文', 'Simplified Chinese'],
  languageEnglish: ['English', 'English'],
  system: ['跟随系统', 'Follow Windows'],
  settings: ['设置', 'Settings'],
  settingsHelp: [
    '主题、语言、SAS 程序路径与窗口行为。',
    'Theme, language, the SAS executable path and window behaviour.',
  ],
  interfaceGroup: ['界面', 'Interface'],
  theme: ['主题', 'Theme'],
  themeDark: ['深色', 'Dark'],
  themeLight: ['浅色', 'Light'],
  sasGroup: ['SAS 程序', 'SAS program'],
  sasPath: ['SAS 程序路径', 'SAS executable path'],
  sasPathHint: [
    '填写 sas.exe 的完整路径；「自动检测」会清除手动指定的位置。',
    'Point at sas.exe directly; Auto-detect clears the manual override.',
  ],
  savePath: ['保存路径', 'Save path'],
  browse: ['浏览…', 'Browse…'],
  autoDetect: ['自动检测', 'Auto-detect'],
  installationAutoDetected: ['已恢复自动检测。', 'Auto-detect restored.'],
  windowGroup: ['启动与窗口', 'Startup and window'],
  startup: ['开机时自动启动', 'Start at login'],
  startupHint: [
    '登录 Windows 后自动运行本程序。',
    'Runs the launcher after you sign in to Windows.',
  ],
  startMinimized: ['启动时最小化到托盘', 'Start minimized in the tray'],
  startMinimizedActive: ['登录后直接进入托盘运行。', 'The login launch goes straight to the tray.'],
  startMinimizedShown: ['登录后正常显示窗口。', 'The login launch shows the window.'],
  closeToTray: ['关闭时最小化到托盘', 'Minimize to the tray on close'],
  closeToTrayHint: [
    '窗口关闭后程序继续在托盘运行。',
    'The app keeps running in the tray after you close it.',
  ],
  quitTitle: ['退出 SAS 启动管理器？', 'Quit SAS Launcher?'],
  quitMessage: [
    '仍有 {count} 个 SAS 会话在运行。退出后它们会继续运行，但不再由本程序管理。',
    '{count} SAS session(s) are still running. They keep running after you quit, but this app no longer manages them.',
  ],
  quitAction: ['退出', 'Quit'],
  discovering: ['正在查找 SAS 配置…', 'Looking for SAS configurations…'],
  runningCount: ['{count} 个会话运行中', '{count} running'],

  closeError: ['关闭错误', 'Dismiss error'],
  closeMessage: ['关闭提示', 'Dismiss message'],
  chooseExe: ['选择 sas.exe', 'Choose sas.exe'],
  pickSas: ['选择已安装的 SAS 程序', 'Select the installed SAS executable'],
  windowsProgram: ['Windows 程序', 'Windows executable'],
  installationUpdated: ['已更新 SAS 安装位置。', 'SAS installation updated.'],
  refresh: ['刷新配置', 'Refresh configurations'],
  refreshed: ['已刷新配置。', 'Configurations refreshed.'],
  archiveNotice: ['旧数据已备份到 {path}', 'Old data was backed up to {path}'],
  openArchive: ['打开备份目录', 'Open backup folder'],

  name: ['项目名称', 'Project name'],
  example: ['例如：项目 A', 'e.g. Project A'],
  requiredField: ['必填', 'Required'],
  optionalField: ['选填', 'Optional'],
  nameRequired: ['请填写项目名称', 'Enter a project name'],
  note: ['备注', 'Notes'],
  noteHint: ['可补充项目用途或其他说明', 'Add a purpose or other details'],
  startupFile: ['启动代码文件', 'Startup code file'],
  startupFileHint: [
    'SAS 启动时自动执行的 autoexec 文件，可留空',
    'Autoexec file SAS runs at launch. Leave it empty to run none',
  ],
  browseStartupFile: ['选择文件…', 'Choose file…'],
  clearStartupFile: ['清除', 'Clear'],
  pickStartupFile: ['选择启动代码文件', 'Select the startup code file'],
  sasProgram: ['SAS 程序', 'SAS program'],
  cancel: ['取消', 'Cancel'],
  save: ['保存', 'Save'],
  launch: ['启动', 'Launch'],
  saveProject: ['保存项目', 'Save project'],
  editProject: ['编辑项目', 'Edit project'],
  newProject: ['新建项目', 'New project'],
  deleteAction: ['删除', 'Delete'],
  deleteTitle: ['删除项目', 'Delete project'],
  deletePrompt: ['删除项目「{name}」？', 'Delete the project “{name}”?'],
  deleteNamed: ['删除 {name}', 'Delete {name}'],
  editNamed: ['编辑 {name}', 'Edit {name}'],
  selectNamed: ['选择项目 {name}', 'Select project {name}'],
  projectSaved: ['项目已保存。', 'Project saved.'],
  projectDeleted: ['项目已删除。', 'Project deleted.'],
  launched: ['已启动「{name}」。', 'Started “{name}”.'],
  noProjects: ['还没有保存的项目', 'No saved projects yet'],
  noProjectsHelp: [
    '保存常用项目的名称和配置，下次直接启动。',
    'Save a project name and configuration to start it next time.',
  ],
  firstProject: ['添加第一个项目', 'Add your first project'],
  checkAll: ['全选', 'Select all'],
  launchSelected: ['启动所选（{count}）', 'Launch selected ({count})'],
  batchFailures: ['部分项目未能启动', 'Some projects could not be started'],
  unavailableProject: ['项目不可用', 'Unavailable project'],
  missingConfig: [
    '配置不可用，请编辑项目重新选择',
    'The configuration is unavailable. Edit the project to choose another one.',
  ],
  configSelection: ['配置选择', 'Configuration'],

  userFolderGroup: ['用户偏好', 'User preference'],
  inheritUserFolderOption: ['继承我的 SAS 用户目录', 'Use my SAS user folder'],
  inheritUserFolderHint: [
    '把 My SAS Files 里调好的窗口布局、功能键与菜单带进这个项目',
    'Carry the layout, function keys and menus tuned in My SAS Files into this project',
  ],
  inheritUserFolderMissing: [
    '未找到 My SAS Files 下的用户偏好目录',
    'No user preference folder under My SAS Files',
  ],
  inheritUserFolderSummary: [
    '将继承 {count} 个文件 · 修改于 {when}',
    'Inherits {count} files · last changed {when}',
  ],
  inheritUserFolderEmpty: [
    '这个文件夹里没有 SAS 设置文件',
    'This folder holds no SAS settings files',
  ],
  inheritConfigSnapshotOption: ['继承某个配置保存的设置', 'Use the settings a configuration saved'],
  inheritConfigSnapshotHint: [
    '沿用该配置保存过的会话设置，编码一定一致；需要先在配置管理里保存过一次会话设置',
    'Reuse the settings that configuration saved, so the encoding always matches. Save a session first',
  ],
  inheritConfigSnapshotEmpty: [
    '没有已保存过会话设置的配置',
    'No configuration has saved session settings yet',
  ],
  noPreferenceOption: ['不使用已有偏好', 'Start without preferences'],
  noPreferenceHint: [
    '从空的 SASUSER 开始，不继承任何偏好；之后在这个项目里调好的界面会留给下次',
    'Start from an empty SASUSER with nothing inherited; the interface you tune here is kept',
  ],
  profileLayersCarryOver: [
    '窗口布局、功能键、菜单与资源管理器视图会沿用你现在的设置。',
    'Your window layout, function keys, menus and Explorer view carry over.',
  ],
  profileLayersAlso: ['还可以带上：', 'Also bring:'],
  profileLayersEnvironment: [
    '环境（库分配、Explorer 选项、打印设置）',
    'Environment (library assignments, Explorer options, print setup)',
  ],
  profileLayersEnvironmentHint: [
    '会把你现在注册的库分配、文件快捷方式与打印设置一起带过来，其中可能指向别的项目的路径',
    'Carries the libraries, file shortcuts and print setup registered now, which may point at another project',
  ],
  profileLayersTemplates: ['ODS 模板与样式', 'Custom ODS templates and styles'],
  browseFolder: ['浏览其他文件夹…', 'Browse for another folder…'],

  followDefaultOption: ['跟随默认配置', 'Follow the default configuration'],
  followDefaultOptionHint: [
    '启动时读取 SAS 安装的默认配置',
    'Reads the SAS installation default when starting',
  ],
  followDefaultOptionHintWith: [
    '启动时读取 SAS 安装的默认配置（{options}）',
    'Reads the SAS installation default when starting ({options})',
  ],
  followDefaultMissing: ['当前未检测到默认配置', 'No default configuration detected'],
  builtInOption: ['内置配置', 'Built-in configuration'],
  builtInOptionHint: [
    '跟到某个 SAS 出厂目录，SAS 升级后可能变化',
    'Follows a factory SAS locale folder; it may change when SAS is updated',
  ],
  userOption: ['用户配置', 'User configuration'],
  userOptionHint: [
    '跟到你自己维护的配置，可单独保存 SAS 设置',
    'Follows a configuration you maintain, with its own saved SAS settings',
  ],
  createConfiguration: ['创建新配置', 'Create a configuration'],
  configurationPickerEmpty: ['没有可选的配置', 'No configurations available'],
  helpHint: ['说明', 'Help'],

  userConfigurations: ['用户配置', 'User configurations'],
  builtInConfigurations: ['内置配置（只读）', 'Built-in configurations (read-only)'],
  noUserConfigurations: ['还没有用户配置', 'No user configurations yet'],
  noUserConfigurationsHelp: [
    '从任一内置配置衍生一份，就能单独调整编码与区域。',
    'Derive one from a built-in configuration to adjust its encoding and locale.',
  ],
  noBuiltInConfigurations: ['没有检测到内置配置', 'No built-in configurations found'],
  configActions: ['配置操作：{name}', 'Actions for {name}'],
  deriveAction: ['从本配置衍生', 'Derive from this configuration'],
  deriveNamed: ['从「{name}」衍生配置', 'Derive a configuration from “{name}”'],
  editSettings: ['编辑设置', 'Edit settings'],
  openSasSettings: ['打开 SAS 调整设置', 'Adjust settings in SAS'],
  deleteConfiguration: ['删除配置', 'Delete configuration'],
  deleteConfigurationTitle: ['删除配置', 'Delete configuration'],
  deleteConfigurationDetail: [
    '删除用户配置「{name}」及它保存的设置？',
    'Delete the user configuration “{name}” and the settings it saved?',
  ],
  configurationDeleted: ['配置已删除。', 'Configuration deleted.'],
  configurationSaved: ['配置已保存。', 'Configuration saved.'],
  configurationCreated: ['配置已创建。', 'Configuration created.'],
  setupSessionRunning: ['设置会话进行中', 'Settings session in progress'],
  saveSessionSettings: ['保存这次的设置', 'Save these settings'],
  discard: ['丢弃', 'Discard'],
  setupStarted: [
    '已打开 SAS 设置会话，改完关闭 SAS 窗口再回来保存。',
    'The SAS settings session is open. Close its window, then come back to save.',
  ],
  setupSaved: ['已保存这次的设置。', 'These settings were saved.'],
  setupDiscarded: ['已丢弃这次的设置。', 'These settings were discarded.'],
  setupDialogTitle: ['打开 SAS 调整设置：{name}', 'Adjust settings in SAS: {name}'],
  setupDialogBody: [
    '会打开一个独立的 SAS 会话。在 SAS 里修改的设置（Preferences、窗口布局、快捷键、Results 选项卡、Explorer 配置、启动时启用的 libref）可以在退出后保存到这个配置。',
    'This opens a separate SAS session. Settings changed there (Preferences, window layout, shortcuts, Results tabs, Explorer settings and start-up librefs) can be saved to this configuration after it exits.',
  ],
  setupDialogConfirm: ['打开 SAS', 'Open SAS'],
  editSettingsTitle: ['编辑设置：{name}', 'Edit settings: {name}'],
  encoding: ['编码', 'Encoding'],
  locale: ['区域', 'Locale'],
  encodingHint: ['例如 EUC-CN、UTF-8', 'e.g. EUC-CN, UTF-8'],
  presetCombination: ['预设组合', 'Preset combination'],
  presetCustom: ['自定义组合', 'Custom combination'],
  localeHint: ['例如 zh_CN、en_US', 'e.g. zh_CN, en_US'],
  editorSettingsNotIncluded: [
    'SAS 的增强编辑器字体/颜色/按键存在 Windows 注册表，不随配置保存',
    'Enhanced Editor fonts, colors and keys live in the Windows registry and are not saved with a configuration',
  ],
  presetVerified: ['已验证', 'Verified'],
  presetFailed: ['上次启动失败', 'Last start failed'],
  presetUnverified: ['未验证', 'Unverified'],
  presetUnverifiedHint: [
    '这个编码与区域组合未经验证，仍可启动。',
    'This encoding and locale combination is unverified; you can still start SAS.',
  ],
  presetFailedHint: ['上次用这个组合启动失败。', 'The last start with this combination failed.'],
  optionPairMissing: ['请填写编码与区域', 'Enter an encoding and a locale'],
  includesSettings: ['含设置', 'Has settings'],
  derivedFrom: ['衍生自 {name}', 'Derived from {name}'],
  usedByProjects: ['被 {count} 个项目使用', 'Used by {count} project(s)'],
  unnamedConfiguration: ['未命名配置', 'Unnamed configuration'],
  noOptions: ['未设置编码/区域', 'No encoding or locale'],
  deleteInUse: ['被项目引用的配置不能删除', 'A configuration used by a project cannot be deleted'],
  returnToDraft: ['← 返回未完成的新增项目', '← Back to the unfinished project'],
  sasUiLanguage: ['SAS 界面语言：{language}', 'SAS interface language: {language}'],

  diagnosticsEmpty: ['还没有会话记录', 'No session records yet'],
  diagnosticsEmptyHelp: [
    '启动项目后，会话的解析链与真实启动参数会记录在这里。',
    'After you start a project, its resolution chain and real launch arguments are recorded here.',
  ],
  clearRecords: ['清空记录', 'Clear records'],
  clearRecordsTitle: ['清空会话记录？', 'Clear session records?'],
  clearRecordsDetail: [
    '将删除全部会话记录与日志副本；运行中的会话保留。',
    'This deletes every session record and log copy. Running sessions are kept.',
  ],
  clearRecordsAction: ['清空记录', 'Clear records'],
  recordsCleared: ['会话记录已清空。', 'Session records cleared.'],
  openLog: ['打开日志', 'Open log'],
  openSasuser: ['打开 SASUSER', 'Open SASUSER'],
  chainHeading: ['解析链', 'Resolution chain'],
  hintsHeading: ['自检提示', 'Preflight hints'],
  argvHeading: ['启动参数', 'Launch arguments'],
  chainThinCfg: ['覆盖配置', 'Override config'],
  chainFactoryCfg: ['基线配置', 'Baseline config'],
  chainEncoding: ['编码', 'Encoding'],
  chainLocale: ['区域', 'Locale'],
  retry: ['重试', 'Retry'],
  profileIncomplete: [
    '还有 {count} 个设置文件没带过来，保存后会补齐',
    'Missing {count} settings files; saving fills them in',
  ],
  chainSasuser: ['SASUSER', 'SASUSER'],
  chainProfile: ['偏好', 'Preference'],
  layerInterface: ['界面设置', 'Interface settings'],
  layerEnvironment: ['环境设置', 'Environment settings'],
  layerTemplates: ['ODS 模板', 'ODS templates'],
  profileSeedValue: ['{layers} · {count} 个文件', '{layers} · {count} files'],
  chainLog: ['日志副本', 'Log copy'],
  chainNone: ['未使用', 'Not used'],
  statusRunning: ['运行中', 'Running'],
  statusExited: ['已退出', 'Exited'],
  statusSetup: ['设置会话', 'Settings session'],
  statusUnknown: ['状态未知', 'Unknown'],
  statusCleanupPending: ['等待回收', 'Cleanup pending'],
  stageWith: ['阶段：{stage}', 'Stage: {stage}'],
  failedStageWith: ['失败阶段：{stage}', 'Failed at {stage}'],
  stageExe: ['启动程序', 'launch executable'],
  stageCfg: ['读取配置', 'read configuration'],
  stageProfile: ['加载设置', 'load settings'],
  stageSpawn: ['创建进程', 'spawn process'],
  stageStarted: ['已启动', 'started'],
  exitCodeValue: ['退出码 {code}', 'Exit code {code}'],
  logCounts: ['ERROR {errors} · WARNING {warnings}', 'ERROR {errors} · WARNING {warnings}'],
  sasuserWritable: ['SASUSER 可写', 'SASUSER is writable'],
  sasuserReadOnly: ['SASUSER 不可写', 'SASUSER is not writable'],
  startedAt: ['启动于 {time}', 'Started {time}'],
  sessionKindProject: ['项目', 'Project'],
  sessionKindSetup: ['设置', 'Settings'],
  expandSession: ['展开 {name} 的详情', 'Expand details for {name}'],
  noLog: ['没有日志副本', 'No log copy'],
  noArgv: ['没有记录启动参数', 'No launch arguments recorded'],
} as const

export type MessageKey = keyof typeof messages
export type Translator = (key: MessageKey, values?: Record<string, string | number>) => string

export function translate(language: Language): Translator {
  return (key, values = {}) =>
    messages[key][language === 'en' ? 1 : 0].replace(/{(\w+)}/g, (_, name: string) =>
      String(values[name] ?? ''),
    )
}

export function issueText(language: Language, error: unknown) {
  const catalog = issues as unknown as Record<string, [string, string]>
  const known =
    error && typeof error === 'object' && 'code' in error && typeof error.code === 'string'
      ? catalog[error.code]
      : undefined
  const detail =
    error && typeof error === 'object' && 'detail' in error && typeof error.detail === 'string'
      ? error.detail
      : typeof error === 'string'
        ? error
        : error instanceof Error
          ? error.message
          : ''
  const title = (known ?? catalog.unexpected)[language === 'en' ? 1 : 0]
  return detail ? `${title} (${detail})` : title
}

export function browserLanguage(): Language {
  return /^zh-(CN|SG|Hans(?:-|$))/i.test(navigator.language) ? 'zh-Hans' : 'en'
}
