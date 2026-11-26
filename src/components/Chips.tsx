import type { Config, Language } from '../api'
import type { Translator } from '../i18n'
import { localeLanguageName } from '../lib/config'

/** Region (as a language name), encoding, and whether the configuration carries a settings snapshot. */
export function Chips({
  config,
  language,
  t,
}: {
  config: Config
  language: Language
  t: Translator
}) {
  const region = localeLanguageName(language, config.locale)
  const interfaceLanguage = localeLanguageName(language, config.uiLanguage)
  return (
    <span className="chips">
      {region && (
        <span
          className="chip"
          title={
            interfaceLanguage
              ? t('config.sasUiLanguage', { language: interfaceLanguage })
              : undefined
          }
        >
          {region}
        </span>
      )}
      {config.encoding && <span className="chip">{config.encoding.toUpperCase()}</span>}
      {config.hasProfile && <span className="chip">{t('config.includesSettings')}</span>}
    </span>
  )
}
