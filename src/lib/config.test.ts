import { describe, expect, it } from 'vitest'
import type { PresetStatus } from '../api'
import { type MessageKey, translate } from '../i18n'
import { builtIn, makeSession, userConfig } from '../test/fixtures'
import {
  chainLines,
  configLabel,
  configReady,
  defaultConfigName,
  formatStartedAt,
  localeLanguageName,
  optionSummary,
  optionsWarning,
  presetLabel,
  sortConfigs,
  usageCount,
} from './config'

const zh = translate('zh-Hans')
const en = translate('en')

describe('default configuration names', () => {
  it('default_names_number_per_source', () => {
    expect(defaultConfigName('u8', [])).toBe('用户配置（衍生自 u8）#1')
    expect(defaultConfigName('u8', ['用户配置（衍生自 u8）#1'])).toBe('用户配置（衍生自 u8）#2')
    // The smallest free index wins, even when later numbers are taken.
    expect(defaultConfigName('u8', ['用户配置（衍生自 u8）#1', '用户配置（衍生自 u8）#3'])).toBe(
      '用户配置（衍生自 u8）#2',
    )
    // Numbering is per source and other sources do not influence it.
    expect(defaultConfigName('磐升瑞祥配置', ['用户配置（衍生自 u8）#1'])).toBe(
      '用户配置（衍生自 磐升瑞祥配置）#1',
    )
  })
})

describe('encoding/locale combination state', () => {
  it('unverified_combinations_are_flagged', () => {
    const cases: {
      encoding: string | null
      locale: string | null
      preset: PresetStatus
      status: PresetStatus
      warning: MessageKey | null
    }[] = [
      {
        encoding: 'EUC-CN',
        locale: 'zh_CN',
        preset: 'verified',
        status: 'verified',
        warning: null,
      },
      {
        encoding: 'UTF-8',
        locale: 'en_US',
        preset: 'unverified',
        status: 'unverified',
        warning: 'presetUnverifiedHint',
      },
      {
        encoding: 'UTF-8',
        locale: 'en_US',
        preset: 'failed',
        status: 'failed',
        warning: 'presetFailedHint',
      },
      {
        encoding: null,
        locale: 'zh_CN',
        preset: 'verified',
        status: 'unverified',
        warning: 'optionPairMissing',
      },
    ]
    for (const entry of cases) {
      const check = optionsWarning(entry.encoding, entry.locale, entry.preset)
      expect(check.status, `${entry.encoding}/${entry.locale}`).toBe(entry.status)
      expect(check.warning, `${entry.encoding}/${entry.locale}`).toBe(entry.warning)
      if (entry.warning) expect(zh(entry.warning).length).toBeGreaterThan(0)
    }
  })

  it('a factory combination is verified without a warning', () => {
    expect(optionsWarning('EUC-CN', 'zh_CN', 'verified')).toEqual({
      status: 'verified',
      warning: null,
    })
  })

  it('an unknown combination keeps its warning copy', () => {
    const check = optionsWarning('UTF-8', 'en_US', 'unverified')
    expect(check.status).toBe('unverified')
    expect(check.warning).not.toBeNull()
    expect(zh(check.warning!)).toContain('未经验证')
    expect(en(check.warning!)).toContain('unverified')
  })
})

describe('labels', () => {
  it('config_label_prefers_the_name_and_falls_back_to_the_code', () => {
    expect(configLabel(zh, builtIn('u8', 'UTF-8', 'zh_CN'))).toBe('u8')
    expect(configLabel(zh, userConfig('cfg-1', '磐升瑞祥配置', 'EUC-CN', 'zh_CN'))).toBe(
      '磐升瑞祥配置',
    )
    expect(configLabel(zh, userConfig('cfg-2', '   ', 'EUC-CN', 'zh_CN'))).toBe('未命名配置')
  })

  it('preset_labels_cover_every_status', () => {
    expect(presetLabel(zh, 'verified')).toBe('已验证')
    expect(presetLabel(zh, 'unverified')).toBe('未验证')
    expect(presetLabel(zh, 'failed')).toBe('上次启动失败')
  })

  it('region_chips_render_a_language_name', () => {
    expect(localeLanguageName('zh-Hans', 'zh_CN')).toBe('简体中文')
    expect(localeLanguageName('zh-Hans', 'zh-TW')).toBe('繁体中文')
    expect(localeLanguageName('zh-Hans', 'en_US')).toBe('英语')
    expect(localeLanguageName('en', 'zh_CN')).toBe('Simplified Chinese')
    expect(localeLanguageName('zh-Hans', null)).toBeNull()
    expect(localeLanguageName('zh-Hans', 'not a locale')).toBe('not a locale')
  })

  it('bare_sas_language_names_render_in_the_app_language', () => {
    // 15 factory configurations carry a language name instead of a code.
    expect(localeLanguageName('zh-Hans', 'German')).toBe('德语')
    expect(localeLanguageName('zh-Hans', 'Spanish')).toBe('西班牙语')
    expect(localeLanguageName('en', 'Spanish')).toBe('Spanish')
    expect(localeLanguageName('zh-Hans', 'Russian')).toBe('俄语')
    // Unknown values are shown verbatim instead of being swallowed.
    expect(localeLanguageName('zh-Hans', 'Klingon')).toBe('Klingon')
  })

  it('option_summary_joins_region_and_encoding', () => {
    expect(optionSummary('zh-Hans', builtIn('zh', 'EUC-CN', 'zh_CN'))).toBe('简体中文 · EUC-CN')
    expect(optionSummary('zh-Hans', null)).toBe('')
  })
})

describe('ordering and lookups', () => {
  it('user_configurations_come_before_built_ins_and_are_sorted_by_name', () => {
    const configs = [
      builtIn('u8', 'UTF-8', 'zh_CN'),
      userConfig('cfg-2', '乙配置', 'EUC-CN', 'zh_CN'),
      builtIn('en', 'wlatin1', 'en_US'),
      userConfig('cfg-1', '甲配置', 'EUC-CN', 'zh_CN'),
    ]
    expect(sortConfigs(configs).map((config) => config.name)).toEqual([
      '甲配置',
      '乙配置',
      'en',
      'u8',
    ])
  })

  it('usage_count_counts_projects_referencing_the_configuration', () => {
    const projects = [
      {
        id: 'p1',
        name: 'A',
        note: null,
        config: { kind: 'user' as const, id: 'cfg-1' },
        profile: null,
      },
      {
        id: 'p2',
        name: 'B',
        note: null,
        config: { kind: 'user' as const, id: 'cfg-2' },
        profile: null,
      },
      {
        id: 'p3',
        name: 'C',
        note: null,
        config: { kind: 'user' as const, id: 'cfg-1' },
        profile: null,
      },
    ]
    expect(usageCount(projects, { kind: 'user', id: 'cfg-1' })).toBe(2)
    expect(usageCount(projects, { kind: 'builtIn', code: 'u8' })).toBe(0)
  })

  it('config_ready_rejects_the_unpicked_user_state', () => {
    expect(configReady(null)).toBe(true)
    expect(configReady({ kind: 'user', id: '' })).toBe(false)
    expect(configReady({ kind: 'user', id: 'cfg-1' })).toBe(true)
    expect(configReady({ kind: 'builtIn', code: 'u8' })).toBe(true)
  })
})

describe('resolution chain', () => {
  it('chain_lists_thin_factory_and_effective_options', () => {
    const session = makeSession({ id: 's1', status: 'running', stage: 'started' })
    const lines = chainLines(session.chain)
    expect(lines.map((line) => line.key)).toEqual([
      'chainThinCfg',
      'chainFactoryCfg',
      'chainEncoding',
      'chainLocale',
      'chainSasuser',
      'chainLog',
    ])
    expect(lines.map((line) => line.value)).toEqual([
      session.chain.thinCfg,
      session.chain.factoryCfg,
      'UTF-8',
      'zh_CN',
      session.chain.sasuser,
      session.chain.log,
    ])
    expect(lines[0].value).toBe('C:\\data\\configurations\\cfg-1\\sasv9.cfg')
    expect(lines[1].value).toBe('C:\\SAS\\nls\\u8\\sasv9.cfg')
  })

  it('a missing thin config stays a null line for the chainNone copy', () => {
    const session = makeSession({ id: 's2' })
    session.chain = { ...session.chain, thinCfg: null, log: null }
    expect(chainLines(session.chain)[0].value).toBeNull()
    expect(zh('chainNone')).toBe('未使用')
  })
})

describe('timestamps', () => {
  it('format_started_at_renders_a_local_date_and_time', () => {
    const stamp = new Date(2026, 10, 23, 12, 30).getTime()
    expect(formatStartedAt(stamp, 'zh-Hans')).toContain('2026')
    expect(formatStartedAt(stamp, 'en')).toContain('2026')
    expect(formatStartedAt(0, 'zh-Hans')).toBe('—')
  })
})
