import { keyOf, type Config } from './api'
import { encodingOf, localeOf } from './config-options'

export type ConfigurationSelection = { encoding: string; locale: string; config: string }
export const emptySelection: ConfigurationSelection = { encoding: '', locale: '', config: '' }
export const defaultSelectionValue = '@default'

export function valueFor(config: Config) {
  return config.customId ? 'custom:' + config.customId : keyOf(config.key)
}

export function configForValue(configs: Config[], value: string, defaultConfig?: Config | null) {
  if (value === defaultSelectionValue) return defaultConfig ?? undefined
  return configs.find(config => valueFor(config) === value)
}

export function selectionFor(config: Config | undefined, useDefault = false): ConfigurationSelection {
  return config ? { encoding: encodingOf(config), locale: localeOf(config), config: useDefault ? defaultSelectionValue : valueFor(config) } : emptySelection
}
