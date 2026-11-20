import { sameKey, type Config, type Language } from './api'
import { sasConfigurationEnglishName, sasConfigurationName, type Translator } from './i18n'

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

export function builtInConfigurationName(language: Language, config: Config) {
  return 'SAS 9.4 (' + sasConfigurationName(language, config.code, config.uiLanguage) + ')'
}

function baseConfigurationLabel(t: Translator, language: Language, config: Config) {
  return config.customId
    ? config.customName || t('numberedConfiguration', { number: config.code })
    : builtInConfigurationName(language, config)
}

export function configurationLabel(t: Translator, config: Config, configs: Config[], language: Language) {
  const label = baseConfigurationLabel(t, language, config)
  const duplicate = config.customId && configs.some(other => !sameKey(config.key, other.key)
    && baseConfigurationLabel(t, language, other) === label
    && localeOf(other) === localeOf(config) && encodingOf(other) === encodingOf(config))
  return duplicate ? label + ' · #' + config.code : label
}

export function orderedConfigs(configs: Config[]) {
  const canonicalCollator = new Intl.Collator('en', { numeric: true, sensitivity: 'base' })
  const localeCollator = new Intl.Collator('en', { numeric: true, sensitivity: 'base' })
  const canonicalName = (config: Config) => {
    if (config.customId) return config.customName || 'Custom Configuration ' + config.code
    return sasConfigurationEnglishName(config.code, config.uiLanguage)
  }
  return [...configs].sort((a, b) => Number(!a.customId) - Number(!b.customId)
    || localeCollator.compare(localeOf(a), localeOf(b))
    || canonicalCollator.compare(canonicalName(a), canonicalName(b))
    || localeCollator.compare(a.code, b.code))
}
