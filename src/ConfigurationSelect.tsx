import { keyOf, sameKey, type Config, type ConfigKey, type Language } from './api'
import { configurationLabel, encodingLabel, encodingOf, localeOf, localeLabel, sasLabel } from './config-options'
import { FieldLabel } from './form-fields'
import type { Translator } from './i18n'

export type ConfigurationSelection = { encoding: string; locale: string; config: string }
export const emptySelection: ConfigurationSelection = { encoding: '', locale: '', config: '' }

export function selectionFor(config: Config | undefined): ConfigurationSelection {
  return config ? { encoding: encodingOf(config), locale: localeOf(config), config: keyOf(config.key) } : emptySelection
}

export function ConfigurationSelect({ selection, onChange, configs, language, defaultKey, busy, t, base = false }: {
  selection: ConfigurationSelection
  onChange: (selection: ConfigurationSelection) => void
  configs: Config[]
  language: Language
  defaultKey: ConfigKey | null
  busy: boolean
  t: Translator
  base?: boolean
}) {
  const available = configs.filter(config => !config.error && config.encoding)
  const encodings = [...new Set(available.map(encodingOf))]
  const forEncoding = available.filter(config => encodingOf(config) === selection.encoding)
  const locales = new Map(forEncoding.map(config => [localeOf(config), localeLabel(t, language, config)]))
  const choices = forEncoding.filter(config => localeOf(config) === selection.locale)
  const selected = choices.find(config => keyOf(config.key) === selection.config)

  function chooseLocale(encoding: string, locale: string) {
    const matches = available.filter(config => encodingOf(config) === encoding && localeOf(config) === locale)
    onChange({ encoding, locale, config: matches.length === 1 ? keyOf(matches[0].key) : '' })
  }

  return <>
    <label><FieldLabel label={t(base ? 'filterEncoding' : 'encoding')} marker={t('requiredField')} />
      <select value={encodings.includes(selection.encoding) ? selection.encoding : ''} required disabled={busy} onChange={event => {
        const encoding = event.target.value
        const matches = [...new Set(available.filter(config => encodingOf(config) === encoding).map(localeOf))]
        chooseLocale(encoding, matches.length === 1 ? matches[0] : '')
      }}>
        <option value="" disabled>{t('chooseEncoding')}</option>
        {encodings.map(encoding => <option key={encoding} value={encoding}>{encodingLabel(t, encoding)}</option>)}
      </select>
    </label>
    <label><FieldLabel label={t('locale')} marker={t('requiredField')} />
      <select value={locales.has(selection.locale) ? selection.locale : ''} required disabled={busy || !locales.size}
        onChange={event => chooseLocale(selection.encoding, event.target.value)}>
        <option value="" disabled>{t('chooseLocale')}</option>
        {[...locales].map(([id, label]) => <option key={id} value={id}>{label}</option>)}
      </select>
    </label>
    <label><FieldLabel label={t(base ? 'baseConfiguration' : 'sasConfig')} marker={t('requiredField')} />
      <select value={selected ? keyOf(selected.key) : ''} required disabled={busy || !choices.length}
        onChange={event => onChange({ ...selection, config: event.target.value })}>
        <option value="" disabled>{t(base ? 'chooseBaseConfiguration' : 'chooseConfig')}</option>
        {choices.map(config => <option key={keyOf(config.key)} value={keyOf(config.key)}>
          {configurationLabel(t, config, choices)} · {sasLabel(t, language, config)}{defaultKey && sameKey(defaultKey, config.key) ? ` (${t('defaultOption')})` : ''}
        </option>)}
      </select>
    </label>
  </>
}
