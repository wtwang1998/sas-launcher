import { createContext, type ReactNode, useContext, useMemo } from 'react'
import type { Language } from '../api'
import { translate, type Translator } from './index'

type I18n = { language: Language; t: Translator }

const I18nContext = createContext<I18n | null>(null)

/**
 * The one place the interface language enters the tree. Pages and controls read it
 * instead of receiving `t` and `language` through every prop list.
 */
export function I18nProvider({ language, children }: { language: Language; children: ReactNode }) {
  const value = useMemo(() => ({ language, t: translate(language) }), [language])
  return <I18nContext.Provider value={value}>{children}</I18nContext.Provider>
}

/** Throws outside a provider, so a missing wrapper cannot silently render Chinese. */
export function useI18n(): I18n {
  const value = useContext(I18nContext)
  if (!value) throw new Error('useI18n must be used inside <I18nProvider>')
  return value
}

export function useT(): Translator {
  return useI18n().t
}
