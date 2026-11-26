import type { Config } from '../api'
import { localeLanguageName } from '../lib/config'
import { useI18n } from '../i18n/provider'

/** Region (as a language name), encoding, and whether the configuration carries a settings snapshot. */
export function Chips({ config }: { config: Config }) {
  const { language, t } = useI18n()
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
