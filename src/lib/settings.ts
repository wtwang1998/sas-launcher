import type { ThemePreference } from '../api'

/**
 * `data-theme` value for the stylesheet. `null` leaves the attribute off so the
 * `prefers-color-scheme` media query decides and keeps following Windows.
 */
export function themeAttribute(preference: ThemePreference): 'dark' | 'light' | null {
  return preference === 'system' ? null : preference
}
