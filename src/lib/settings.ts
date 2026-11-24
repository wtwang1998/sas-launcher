import type { ThemePreference } from '../api'

/**
 * `data-theme` value for the stylesheet. `null` leaves the attribute off so the
 * `prefers-color-scheme` media query decides and keeps following Windows.
 */
export function themeAttribute(preference: ThemePreference): 'dark' | 'light' | null {
  return preference === 'system' ? null : preference
}

/**
 * Starting minimized only happens when the login entry is registered: the flag
 * itself is kept, so re-enabling the login entry restores the old behaviour.
 */
export function startMinimizedActive(autostart: boolean, startMinimized: boolean): boolean {
  return autostart && startMinimized
}
