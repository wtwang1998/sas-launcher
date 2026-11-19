import { sameKey, type Config, type ConfigKey, type Language } from './api'
import type { Translator } from './i18n'

export const encodingOf = (config: Config) => config.encoding?.toLowerCase() ?? ''

export function encodingLabel(t: Translator, encoding: string | null) {
  return encoding?.toUpperCase() || t('unknownEncoding')
}

export function sasLabel(t: Translator, language: Language, config: Config) {
  if (!config.uiLanguage) return t('unknownSas', { code: config.code })
  const locale = new Intl.DisplayNames([language], { type: 'language', languageDisplay: 'standard' })
  return config.uiLanguage === 'zh-Hans'
    ? language === 'en' ? 'Simplified Chinese' : '简体中文'
    : config.uiLanguage === 'zh-Hant'
      ? language === 'en' ? 'Traditional Chinese' : '繁体中文'
      : locale.of(config.uiLanguage) ?? config.uiLanguage
}


export const localeOf = (config: Config) => config.locale?.trim().replaceAll('-', '_').toLowerCase() || 'unknown'

export function localeLabel(t: Translator, language: Language, config: Config) {
  const value = config.locale?.trim()
  if (!value) return t('unknownLocale')
  if (!/^[a-z]{2,3}[-_][a-z]{2}$/i.test(value)) return value
  const [lang, region] = value.split(/[-_]/)
  const code = lang.toLowerCase() + '_' + region.toUpperCase()
  const label = new Intl.DisplayNames([language], { type: 'language', fallback: 'none' }).of(code.replace('_', '-'))
  return label ? label + ' · ' + code : code
}

function baseConfigurationLabel(t: Translator, config: Config) {
  return config.customId
    ? config.customName || t('numberedConfiguration', { number: config.code })
    : t('factoryConfiguration', { code: config.code })
}

export function configurationLabel(t: Translator, config: Config, configs: Config[]) {
  const label = baseConfigurationLabel(t, config)
  const duplicate = config.customId && configs.some(other => !sameKey(config.key, other.key)
    && baseConfigurationLabel(t, other) === label
    && localeOf(other) === localeOf(config) && encodingOf(other) === encodingOf(config))
  return duplicate ? `${label} · #${config.code}` : label
}

function naturalLanguage(tag: string) {
  try {
    const locale = new Intl.Locale(tag).maximize()
    return locale.language === 'zh' ? `zh-${locale.script}` : locale.language
  } catch {
    return tag
  }
}

export function orderedConfigs(configs: Config[], system: string, defaultKey: ConfigKey | null) {
  const preferred = naturalLanguage(system)
  function priority(config: Config) {
    return [
      Number(!!config.error),
      Number(config.uiLanguage !== preferred),
      Number(!defaultKey || !sameKey(defaultKey, config.key)),
      Number(encodingOf(config) !== 'utf-8'),
    ]
  }
  return [...configs].sort((a, b) => {
    const left = priority(a)
    const right = priority(b)
    return left.map((value, index) => value - right[index]).find(value => value !== 0)
      ?? a.code.localeCompare(b.code)
  })
}
