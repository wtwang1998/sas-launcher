import { Info } from 'lucide-react'
import { useId } from 'react'
import type { Config, Language } from './api'
import { configurationLabel, encodingLabel, encodingOf, localeOf, localeLabel } from './config-options'
import { ConfigurationPicker } from './ConfigurationPicker'
import { defaultSelectionValue, valueFor, type ConfigurationSelection } from './configuration-selection'
import { FieldLabel } from './form-fields'
import type { Translator } from './i18n'

export function ConfigurationSelect({ selection, onChange, configs, language, defaultConfig, busy, t, base = false }: {
  selection: ConfigurationSelection
  onChange: (selection: ConfigurationSelection) => void
  configs: Config[]
  language: Language
  defaultConfig?: Config | null
  busy: boolean
  t: Translator
  base?: boolean
}) {
  const helpId = useId()
  const available = configs.filter(config => !config.error && config.encoding && config.locale)
  const defaultAvailable = !base && defaultConfig && !defaultConfig.error && defaultConfig.encoding && defaultConfig.locale
    ? defaultConfig
    : undefined
  const localeConfigs = base ? available.filter(config => !config.customId) : available
  const encodings = [...new Set([...localeConfigs.map(encodingOf), ...(defaultAvailable ? [encodingOf(defaultAvailable)] : [])])]
  const locales = new Map(localeConfigs
    .filter(config => encodingOf(config) === selection.encoding)
    .map(config => [localeOf(config), localeLabel(t, language, config)]))
  if (defaultAvailable && encodingOf(defaultAvailable) === selection.encoding) {
    locales.set(localeOf(defaultAvailable), localeLabel(t, language, defaultAvailable))
  }
  const following = !base && selection.config === defaultSelectionValue && !!defaultAvailable
  const choices = base ? available.filter(config => !config.customId) : available

  return <>
    <label><FieldLabel label={t(base ? 'filterEncoding' : 'encoding')} marker={t('requiredField')} />
      <select value={encodings.includes(selection.encoding) ? selection.encoding : ''} required disabled={busy || following} onChange={event => {
        const encoding = event.target.value
        const matches = localeConfigs.filter(config => encodingOf(config) === encoding).map(localeOf)
        const defaultLocale = defaultAvailable && encodingOf(defaultAvailable) === encoding ? [localeOf(defaultAvailable)] : []
        const localeChoices = [...new Set([...matches, ...defaultLocale])]
        onChange({ ...selection, encoding, locale: localeChoices.length === 1 ? localeChoices[0] : selection.locale })
      }}>
        <option value="" disabled>{t('chooseEncoding')}</option>
        {encodings.map(encoding => <option key={encoding} value={encoding}>{encodingLabel(t, encoding)}</option>)}
      </select>
    </label>
    <label><FieldLabel label={t('locale')} marker={t('requiredField')} />
      <select value={locales.has(selection.locale) ? selection.locale : ''} required disabled={busy || !locales.size || following}
        onChange={event => onChange({ ...selection, locale: event.target.value })}>
        <option value="" disabled>{t('chooseLocale')}</option>
        {[...locales].map(([id, label]) => <option key={id} value={id}>{label}</option>)}
      </select>
    </label>
    {base ? <label><FieldLabel label={t('baseConfiguration')} marker={t('requiredField')} />
      <select value={selection.config} required disabled={busy || !choices.length} onChange={event => {
        const config = choices.find(item => valueFor(item) === event.target.value)
        if (config) onChange({
          ...selection,
          config: valueFor(config),
          encoding: config.encoding ? encodingOf(config) : selection.encoding,
          locale: config.locale ? localeOf(config) : selection.locale,
        })
      }}>
        <option value="" disabled>{t('chooseBaseConfiguration')}</option>
        {choices.map(config => <option key={valueFor(config)} value={valueFor(config)}>
          {configurationLabel(t, config, configs, language)}
        </option>)}
      </select>
    </label> : <div className="editor-field">
      <div className="field-label-row">
        <FieldLabel label={t('sasConfig')} marker={t('requiredField')} />
        {defaultAvailable && <span className="follow-default">
          <label className="follow-default-toggle">
            <input type="checkbox" checked={following} disabled={busy} onChange={event => {
              if (event.target.checked) {
                onChange({ encoding: encodingOf(defaultAvailable), locale: localeOf(defaultAvailable), config: defaultSelectionValue })
              } else {
                onChange({ encoding: encodingOf(defaultAvailable), locale: localeOf(defaultAvailable), config: valueFor(defaultAvailable) })
              }
            }} />
            {t('followDefault')}
          </label>
          <span className="setup-tooltip">
            <button type="button" className="info-button" aria-label={t('helpHint')} aria-describedby={helpId}><Info size={10} /></button>
            <span className="setup-tip" role="tooltip" id={helpId}>{t('followDefaultHelp')}</span>
          </span>
        </span>}
      </div>
      {following
        ? <p className="field-hint follow-default-current">{t('currentDefault', { configuration: configurationLabel(t, defaultAvailable!, configs, language) })}</p>
        : <ConfigurationPicker value={selection.config} configs={available} language={language} t={t} disabled={busy}
          ariaLabel={t('sasConfig')} onChange={value => {
            const config = available.find(item => valueFor(item) === value)
            onChange(config
              ? { ...selection, config: value, encoding: encodingOf(config), locale: localeOf(config) }
              : { ...selection, config: value })
          }} />}
    </div>}
  </>
}
