import type { Language } from './api'
import issues from './issue-messages.json'

export const messages = {
  'app.title': ['SAS 启动管理器', 'SAS Launcher'],
  'app.navigation': ['主导航', 'Main navigation'],
  'nav.projects': ['项目', 'Projects'],
  'nav.configurations': ['配置管理', 'Configurations'],
  'nav.sessions': ['会话诊断', 'Session Diagnostics'],
  'nav.projectsHelp': [
    '保存常用项目的配置，一键启动 SAS。',
    'Save a configuration for each project and start SAS in one click.',
  ],
  'nav.configurationsHelp': [
    '维护用户配置的编码与区域；内置配置只读。',
    'Adjust the encoding and locale of your own configurations. Built-in configurations are read-only.',
  ],
  'nav.sessionsHelp': [
    '查看会话状态、解析链与真实启动参数。',
    'Review session status, resolution chains and real launch arguments.',
  ],

  'settings.language': ['应用语言', 'App language'],
  'settings.languageZhHans': ['简体中文', 'Simplified Chinese'],
  'settings.languageEnglish': ['English', 'English'],
  'settings.system': ['跟随系统', 'Follow Windows'],
  'nav.settings': ['设置', 'Settings'],
  'nav.settingsHelp': [
    '主题、语言、SAS 程序路径与窗口行为。',
    'Theme, language, the SAS executable path and window behaviour.',
  ],
  'settings.interfaceGroup': ['界面', 'Interface'],
  'settings.theme': ['主题', 'Theme'],
  'settings.themeDark': ['深色', 'Dark'],
  'settings.themeLight': ['浅色', 'Light'],
  'settings.sasGroup': ['SAS 程序', 'SAS program'],
  'settings.sasPath': ['SAS 程序路径', 'SAS executable path'],
  'settings.sasPathHint': [
    '填写 sas.exe 的完整路径；「自动检测」会清除手动指定的位置。',
    'Point at sas.exe directly; Auto-detect clears the manual override.',
  ],
  'settings.savePath': ['保存路径', 'Save path'],
  'common.browse': ['浏览…', 'Browse…'],
  'common.autoDetect': ['自动检测', 'Auto-detect'],
  'settings.installationAutoDetected': ['已恢复自动检测。', 'Auto-detect restored.'],
  'settings.windowGroup': ['启动与窗口', 'Startup and window'],
  'settings.startup': ['开机时自动启动', 'Start at login'],
  'settings.startupHint': [
    '登录 Windows 后自动运行本程序。',
    'Runs the launcher after you sign in to Windows.',
  ],
  'settings.startMinimized': ['启动时最小化到托盘', 'Start minimized in the tray'],
  'settings.startMinimizedActive': [
    '登录后直接进入托盘运行。',
    'The login launch goes straight to the tray.',
  ],
  'settings.startMinimizedShown': ['登录后正常显示窗口。', 'The login launch shows the window.'],
  'settings.closeToTray': ['关闭时最小化到托盘', 'Minimize to the tray on close'],
  'settings.closeToTrayHint': [
    '窗口关闭后程序继续在托盘运行。',
    'The app keeps running in the tray after you close it.',
  ],
  'quit.quitTitle': ['退出 SAS 启动管理器？', 'Quit SAS Launcher?'],
  'quit.quitMessage': [
    '仍有 {count} 个 SAS 会话在运行。退出后它们会继续运行，但不再由本程序管理。',
    '{count} SAS session(s) are still running. They keep running after you quit, but this app no longer manages them.',
  ],
  'quit.quitAction': ['退出', 'Quit'],
  'app.discovering': ['正在查找 SAS 配置…', 'Looking for SAS configurations…'],
  'app.runningCount': ['{count} 个会话运行中', '{count} running'],

  'notice.closeError': ['关闭错误', 'Dismiss error'],
  'notice.closeMessage': ['关闭提示', 'Dismiss message'],
  'dialog.chooseExe': ['选择 sas.exe', 'Choose sas.exe'],
  'dialog.pickSas': ['选择已安装的 SAS 程序', 'Select the installed SAS executable'],
  'dialog.windowsProgram': ['Windows 程序', 'Windows executable'],
  'notice.installationUpdated': ['已更新 SAS 安装位置。', 'SAS installation updated.'],
  'common.refresh': ['刷新配置', 'Refresh configurations'],
  'notice.refreshed': ['已刷新配置。', 'Configurations refreshed.'],
  'notice.archiveNotice': ['旧数据已备份到 {path}', 'Old data was backed up to {path}'],
  'common.openArchive': ['打开备份目录', 'Open backup folder'],

  'project.name': ['项目名称', 'Project name'],
  'project.example': ['例如：项目 A', 'e.g. Project A'],
  'form.requiredField': ['必填', 'Required'],
  'form.optionalField': ['选填', 'Optional'],
  'project.nameRequired': ['请填写项目名称', 'Enter a project name'],
  'project.note': ['备注', 'Notes'],
  'project.noteHint': ['可补充项目用途或其他说明', 'Add a purpose or other details'],
  'project.startupFile': ['启动代码文件', 'Startup code file'],
  'project.startupFileHint': [
    'SAS 启动时自动执行的 autoexec 文件，可留空',
    'Autoexec file SAS runs at launch. Leave it empty to run none',
  ],
  'dialog.browseStartupFile': ['选择文件…', 'Choose file…'],
  'common.clearStartupFile': ['清除', 'Clear'],
  'dialog.pickStartupFile': ['选择启动代码文件', 'Select the startup code file'],
  'project.sasProgram': ['SAS 程序', 'SAS program'],
  'project.startupExposesPath': [
    '向会话提供启动文件路径',
    'Tell the session where the startup file is',
  ],
  'project.migrateStartup': ['迁移启动文件', 'Migrate the startup script'],
  'notice.startupMigrated': [
    '启动文件已迁移，原文件保留为 .bak',
    'Startup script migrated; the original is kept as .bak',
  ],
  'project.startupNeedsMigrationHint': [
    '这个脚本还在读 SAS_EXECFILEPATH，自动执行时拿不到值。',
    'This script still reads SAS_EXECFILEPATH, which is empty when it runs at startup.',
  ],
  'project.startupExposesPathHint': [
    '打开后脚本里可以用 &SASLAUNCHER_STARTUP_FILE 拿到本文件的路径；靠自身位置推导项目路径时需要，路径写死的不用。',
    'When on, the script can read &SASLAUNCHER_STARTUP_FILE; turn it on when the script derives paths from its own location. Hard-coded paths need neither.',
  ],
  'common.cancel': ['取消', 'Cancel'],
  'common.save': ['保存', 'Save'],
  'common.launch': ['启动', 'Launch'],
  'project.saveProject': ['保存项目', 'Save project'],
  'project.editProject': ['编辑项目', 'Edit project'],
  'project.newProject': ['新建项目', 'New project'],
  'common.deleteAction': ['删除', 'Delete'],
  'project.deleteTitle': ['删除项目', 'Delete project'],
  'project.deletePrompt': ['删除项目「{name}」？', 'Delete the project “{name}”?'],
  'project.deleteNamed': ['删除 {name}', 'Delete {name}'],
  'project.editNamed': ['编辑 {name}', 'Edit {name}'],
  'project.selectNamed': ['选择项目 {name}', 'Select project {name}'],
  'notice.projectSaved': ['项目已保存。', 'Project saved.'],
  'notice.projectDeleted': ['项目已删除。', 'Project deleted.'],
  'notice.launched': ['已启动「{name}」。', 'Started “{name}”.'],
  'project.noProjects': ['还没有保存的项目', 'No saved projects yet'],
  'project.noProjectsHelp': [
    '保存常用项目的名称和配置，下次直接启动。',
    'Save a project name and configuration to start it next time.',
  ],
  'project.firstProject': ['添加第一个项目', 'Add your first project'],
  'project.checkAll': ['全选', 'Select all'],
  'project.launchSelected': ['启动所选（{count}）', 'Launch selected ({count})'],
  'project.batchFailures': ['部分项目未能启动', 'Some projects could not be started'],
  'project.unavailableProject': ['项目不可用', 'Unavailable project'],
  'project.missingConfig': [
    '配置不可用，请编辑项目重新选择',
    'The configuration is unavailable. Edit the project to choose another one.',
  ],
  'project.configSelection': ['配置选择', 'Configuration'],

  'profile.userFolderGroup': ['用户偏好', 'User preference'],
  'profile.inheritUserFolderOption': ['继承我的 SAS 用户目录', 'Use my SAS user folder'],
  'profile.inheritUserFolderHint': [
    '把 My SAS Files 里调好的窗口布局、功能键与菜单带进这个项目',
    'Carry the layout, function keys and menus tuned in My SAS Files into this project',
  ],
  'profile.inheritUserFolderMissing': [
    '未找到 My SAS Files 下的用户偏好目录',
    'No user preference folder under My SAS Files',
  ],
  'profile.inheritUserFolderSummary': [
    '将继承 {count} 个文件 · 修改于 {when}',
    'Inherits {count} files · last changed {when}',
  ],
  'profile.inheritUserFolderEmpty': [
    '这个文件夹里没有 SAS 设置文件',
    'This folder holds no SAS settings files',
  ],
  'profile.inheritConfigSnapshotOption': [
    '继承某个配置保存的设置',
    'Use the settings a configuration saved',
  ],
  'profile.inheritConfigSnapshotHint': [
    '沿用该配置保存过的会话设置，编码一定一致；需要先在配置管理里保存过一次会话设置',
    'Reuse the settings that configuration saved, so the encoding always matches. Save a session first',
  ],
  'profile.inheritConfigSnapshotEmpty': [
    '没有已保存过会话设置的配置',
    'No configuration has saved session settings yet',
  ],
  'profile.noPreferenceOption': ['不使用已有偏好', 'Start without preferences'],
  'profile.noPreferenceHint': [
    '从空的 SASUSER 开始，不继承任何偏好；之后在这个项目里调好的界面会留给下次',
    'Start from an empty SASUSER with nothing inherited; the interface you tune here is kept',
  ],
  'profile.profileLayersCarryOver': [
    '窗口布局、功能键、菜单与资源管理器视图会沿用你现在的设置。',
    'Your window layout, function keys, menus and Explorer view carry over.',
  ],
  'profile.profileLayersAlso': ['还可以带上：', 'Also bring:'],
  'profile.profileLayersEnvironment': [
    '环境（库分配、Explorer 选项、打印设置）',
    'Environment (library assignments, Explorer options, print setup)',
  ],
  'profile.profileLayersEnvironmentHint': [
    '会把你现在注册的库分配、文件快捷方式与打印设置一起带过来，其中可能指向别的项目的路径',
    'Carries the libraries, file shortcuts and print setup registered now, which may point at another project',
  ],
  'profile.profileLayersTemplates': ['ODS 模板与样式', 'Custom ODS templates and styles'],
  'dialog.browseFolder': ['浏览其他文件夹…', 'Browse for another folder…'],

  'configChoice.followDefaultOption': ['跟随默认配置', 'Follow the default configuration'],
  'configChoice.followDefaultOptionHint': [
    '启动时读取 SAS 安装的默认配置',
    'Reads the SAS installation default when starting',
  ],
  'configChoice.followDefaultOptionHintWith': [
    '启动时读取 SAS 安装的默认配置（{options}）',
    'Reads the SAS installation default when starting ({options})',
  ],
  'configChoice.followDefaultMissing': [
    '当前未检测到默认配置',
    'No default configuration detected',
  ],
  'configChoice.builtInOption': ['内置配置', 'Built-in configuration'],
  'configChoice.builtInOptionHint': [
    '跟到某个 SAS 出厂目录，SAS 升级后可能变化',
    'Follows a factory SAS locale folder; it may change when SAS is updated',
  ],
  'configChoice.userOption': ['用户配置', 'User configuration'],
  'configChoice.userOptionHint': [
    '跟到你自己维护的配置，可单独保存 SAS 设置',
    'Follows a configuration you maintain, with its own saved SAS settings',
  ],
  'configChoice.createConfiguration': ['创建新配置', 'Create a configuration'],
  'configChoice.configurationPickerEmpty': ['没有可选的配置', 'No configurations available'],
  'common.helpHint': ['说明', 'Help'],

  'configurations.userConfigurations': ['用户配置', 'User configurations'],
  'configurations.builtInConfigurations': [
    '内置配置（只读）',
    'Built-in configurations (read-only)',
  ],
  'configurations.noUserConfigurations': ['还没有用户配置', 'No user configurations yet'],
  'configurations.noUserConfigurationsHelp': [
    '从任一内置配置衍生一份，就能单独调整编码与区域。',
    'Derive one from a built-in configuration to adjust its encoding and locale.',
  ],
  'configurations.noBuiltInConfigurations': [
    '没有检测到内置配置',
    'No built-in configurations found',
  ],
  'configurations.configActions': ['配置操作：{name}', 'Actions for {name}'],
  'configurations.deriveAction': ['从本配置衍生', 'Derive from this configuration'],
  'configurations.deriveNamed': ['从「{name}」衍生配置', 'Derive a configuration from “{name}”'],
  'configurations.editSettings': ['编辑设置', 'Edit settings'],
  'configurations.openSasSettings': ['打开 SAS 调整设置', 'Adjust settings in SAS'],
  'configurations.deleteConfiguration': ['删除配置', 'Delete configuration'],
  'configurations.deleteConfigurationTitle': ['删除配置', 'Delete configuration'],
  'configurations.deleteConfigurationDetail': [
    '删除用户配置「{name}」及它保存的设置？',
    'Delete the user configuration “{name}” and the settings it saved?',
  ],
  'notice.configurationDeleted': ['配置已删除。', 'Configuration deleted.'],
  'notice.configurationSaved': ['配置已保存。', 'Configuration saved.'],
  'notice.configurationCreated': ['配置已创建。', 'Configuration created.'],
  'configurations.setupSessionRunning': ['设置会话进行中', 'Settings session in progress'],
  'configurations.saveSessionSettings': ['保存这次的设置', 'Save these settings'],
  'common.discard': ['丢弃', 'Discard'],
  'notice.setupStarted': [
    '已打开 SAS 设置会话，改完关闭 SAS 窗口再回来保存。',
    'The SAS settings session is open. Close its window, then come back to save.',
  ],
  'notice.setupSaved': ['已保存这次的设置。', 'These settings were saved.'],
  'notice.setupDiscarded': ['已丢弃这次的设置。', 'These settings were discarded.'],
  'configurations.setupDialogTitle': [
    '打开 SAS 调整设置：{name}',
    'Adjust settings in SAS: {name}',
  ],
  'configurations.setupDialogBody': [
    '会打开一个独立的 SAS 会话。在 SAS 里修改的设置（Preferences、窗口布局、快捷键、Results 选项卡、Explorer 配置、启动时启用的 libref）可以在退出后保存到这个配置。',
    'This opens a separate SAS session. Settings changed there (Preferences, window layout, shortcuts, Results tabs, Explorer settings and start-up librefs) can be saved to this configuration after it exits.',
  ],
  'configurations.setupDialogConfirm': ['打开 SAS', 'Open SAS'],
  'configurations.editSettingsTitle': ['编辑设置：{name}', 'Edit settings: {name}'],
  'config.encoding': ['编码', 'Encoding'],
  'config.locale': ['区域', 'Locale'],
  'config.encodingHint': ['例如 EUC-CN、UTF-8', 'e.g. EUC-CN, UTF-8'],
  'config.presetCombination': ['预设组合', 'Preset combination'],
  'config.presetCustom': ['自定义组合', 'Custom combination'],
  'config.localeHint': ['例如 zh_CN、en_US', 'e.g. zh_CN, en_US'],
  'config.editorSettingsNotIncluded': [
    'SAS 的增强编辑器字体/颜色/按键存在 Windows 注册表，不随配置保存',
    'Enhanced Editor fonts, colors and keys live in the Windows registry and are not saved with a configuration',
  ],
  'config.presetVerified': ['已验证', 'Verified'],
  'config.presetFailed': ['上次启动失败', 'Last start failed'],
  'config.presetUnverified': ['未验证', 'Unverified'],
  'config.presetUnverifiedHint': [
    '这个编码与区域组合未经验证，仍可启动。',
    'This encoding and locale combination is unverified; you can still start SAS.',
  ],
  'config.presetFailedHint': [
    '上次用这个组合启动失败。',
    'The last start with this combination failed.',
  ],
  'config.optionPairMissing': ['请填写编码与区域', 'Enter an encoding and a locale'],
  'config.includesSettings': ['含设置', 'Has settings'],
  'config.derivedFrom': ['衍生自 {name}', 'Derived from {name}'],
  'config.usedByProjects': ['被 {count} 个项目使用', 'Used by {count} project(s)'],
  'config.unnamedConfiguration': ['未命名配置', 'Unnamed configuration'],
  'config.noOptions': ['未设置编码/区域', 'No encoding or locale'],
  'config.deleteInUse': [
    '被项目引用的配置不能删除',
    'A configuration used by a project cannot be deleted',
  ],
  'project.returnToDraft': ['← 返回未完成的新增项目', '← Back to the unfinished project'],
  'config.sasUiLanguage': ['SAS 界面语言：{language}', 'SAS interface language: {language}'],

  'sessions.diagnosticsEmpty': ['还没有会话记录', 'No session records yet'],
  'sessions.diagnosticsEmptyHelp': [
    '启动项目后，会话的解析链与真实启动参数会记录在这里。',
    'After you start a project, its resolution chain and real launch arguments are recorded here.',
  ],
  'sessions.clearRecords': ['清空记录', 'Clear records'],
  'sessions.clearRecordsTitle': ['清空会话记录？', 'Clear session records?'],
  'sessions.clearRecordsDetail': [
    '将删除全部会话记录与日志副本；运行中的会话保留。',
    'This deletes every session record and log copy. Running sessions are kept.',
  ],
  'sessions.clearRecordsAction': ['清空记录', 'Clear records'],
  'notice.recordsCleared': ['会话记录已清空。', 'Session records cleared.'],
  'sessions.openLog': ['打开日志', 'Open log'],
  'sessions.openSasuser': ['打开 SASUSER', 'Open SASUSER'],
  'session.chainHeading': ['解析链', 'Resolution chain'],
  'session.hintsHeading': ['自检提示', 'Preflight hints'],
  'session.argvHeading': ['启动参数', 'Launch arguments'],
  'session.chainThinCfg': ['覆盖配置', 'Override config'],
  'session.chainFactoryCfg': ['基线配置', 'Baseline config'],
  'session.chainEncoding': ['编码', 'Encoding'],
  'session.chainLocale': ['区域', 'Locale'],
  'common.retry': ['重试', 'Retry'],
  'session.profileIncomplete': [
    '还有 {count} 个设置文件没带过来，保存后会补齐',
    'Missing {count} settings files; saving fills them in',
  ],
  'session.chainSasuser': ['SASUSER', 'SASUSER'],
  'session.chainProfile': ['偏好', 'Preference'],
  'session.layerInterface': ['界面设置', 'Interface settings'],
  'session.layerEnvironment': ['环境设置', 'Environment settings'],
  'session.layerTemplates': ['ODS 模板', 'ODS templates'],
  'session.profileSeedValue': ['{layers} · {count} 个文件', '{layers} · {count} files'],
  'session.chainLog': ['日志副本', 'Log copy'],
  'session.chainNone': ['未使用', 'Not used'],
  'session.statusRunning': ['运行中', 'Running'],
  'session.statusExited': ['已退出', 'Exited'],
  'session.statusSetup': ['设置会话', 'Settings session'],
  'session.statusUnknown': ['状态未知', 'Unknown'],
  'session.statusCleanupPending': ['等待回收', 'Cleanup pending'],
  'session.stageWith': ['阶段：{stage}', 'Stage: {stage}'],
  'session.failedStageWith': ['失败阶段：{stage}', 'Failed at {stage}'],
  'session.stageExe': ['启动程序', 'launch executable'],
  'session.stageCfg': ['读取配置', 'read configuration'],
  'session.stageProfile': ['加载设置', 'load settings'],
  'session.stageSpawn': ['创建进程', 'spawn process'],
  'session.stageStarted': ['已启动', 'started'],
  'session.exitCodeValue': ['退出码 {code}', 'Exit code {code}'],
  'session.logCounts': [
    'ERROR {errors} · WARNING {warnings}',
    'ERROR {errors} · WARNING {warnings}',
  ],
  'session.sasuserWritable': ['SASUSER 可写', 'SASUSER is writable'],
  'session.sasuserReadOnly': ['SASUSER 不可写', 'SASUSER is not writable'],
  'session.startedAt': ['启动于 {time}', 'Started {time}'],
  'session.sessionKindProject': ['项目', 'Project'],
  'session.sessionKindSetup': ['设置', 'Settings'],
  'session.expandSession': ['展开 {name} 的详情', 'Expand details for {name}'],
  'session.noLog': ['没有日志副本', 'No log copy'],
  'session.noArgv': ['没有记录启动参数', 'No launch arguments recorded'],
  'nav.about': ['关于', 'About'],
  'nav.aboutHelp': [
    '安装来源、构建版本与许可证。',
    'Where this build came from, which version it is, and its license.',
  ],
  'about.versionGroup': ['版本信息', 'Version'],
  'about.buildCommit': ['构建提交', 'Build commit'],
  'about.copyVersion': ['复制版本信息', 'Copy version info'],
  'notice.copied': ['已复制', 'Copied'],
  'about.linksGroup': ['链接', 'Links'],
  'about.repository': ['源码仓库', 'Source repository'],
  'about.reportIssue': ['报告问题', 'Report an issue'],
  'about.updateGroup': ['更新', 'Updates'],
  'about.checkUpdate': ['检查更新', 'Check for updates'],
  'about.updateChecking': ['正在检查更新…', 'Checking for updates…'],
  'about.updateLatest': [
    '已是最新版本（{version}）。',
    'You are on the latest version ({version}).',
  ],
  'about.updateAvailable': ['有新版本 {version}。', 'Version {version} is available.'],
  'about.updatePreRelease': ['有预发布版 {version}。', 'Pre-release {version} is available.'],
  'about.updateTimeout': ['检查超时，可稍后重试。', 'The check timed out. Try again later.'],
  'about.updateOffline': [
    '无法连接 GitHub，请检查网络后重试。',
    'Cannot reach GitHub. Check your connection and try again.',
  ],
  'about.updateRateLimited': ['请求过于频繁，请稍后再试。', 'Too many requests. Try again later.'],
  'about.updateUnreadable': ['服务器的响应无法解析。', 'The response could not be read.'],
  'common.openRelease': ['打开发布页', 'Open the release page'],
} as const

export type MessageKey = keyof typeof messages
export type Translator = (key: MessageKey, values?: Record<string, string | number>) => string

/**
 * One key in one language. The closure `translate` below and every hook built on it
 * come through here, so a table-driven test can drive the same seam the UI uses.
 */
export function translateKey(
  language: Language,
  key: MessageKey,
  values: Record<string, string | number> = {},
): string {
  return messages[key][language === 'en' ? 1 : 0].replace(/{(\w+)}/g, (_, name: string) =>
    String(values[name] ?? ''),
  )
}

export function translate(language: Language): Translator {
  return (key, values) => translateKey(language, key, values)
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
