import { expect, it } from 'vitest'

/**
 * The NSIS languages tauri 2 ships copy for. A name outside this set makes the build
 * succeed while the installer shows raw `$(createDesktop)`-style strings instead.
 */
const translatedLanguages = new Set([
  'Arabic',
  'Bulgarian',
  'Dutch',
  'English',
  'Farsi',
  'French',
  'German',
  'Hebrew',
  'Italian',
  'Japanese',
  'Korean',
  'Norwegian',
  'Portuguese',
  'PortugueseBR',
  'Russian',
  'SimpChinese',
  'Spanish',
  'SpanishInternational',
  'Swedish',
  'TradChinese',
  'Turkish',
  'Ukrainian',
  'Vietnamese',
])

const files = import.meta.glob('/src-tauri/**/*.{json,nsh}', {
  query: '?raw',
  import: 'default',
  eager: true,
}) as Record<string, string>

const read = (path: string) => {
  const text = files[path]
  if (text === undefined) throw new Error(`${path} was not found`)
  return text
}

it('the_installer_only_offers_languages_tauri_translates', () => {
  const config = JSON.parse(read('/src-tauri/tauri.conf.json'))
  const nsis = config.bundle.windows.nsis

  expect(nsis.displayLanguageSelector).toBe(true)
  expect(nsis.languages[0], 'the first language is the fallback').toBe('English')
  expect(nsis.languages.filter((name: string) => !translatedLanguages.has(name))).toEqual([])
  expect(nsis.languages).toContain('SimpChinese')
  expect(nsis.languages).toContain('TradChinese')
})

it('the_install_hook_has_copy_for_the_languages_it_offers', () => {
  const hook = read('/src-tauri/windows/preserve-install-location.nsh')
  // makensis runs with -INPUTCHARSET UTF8, so the hook must stay UTF-8 without a BOM.
  expect(hook.charCodeAt(0)).not.toBe(0xfeff)

  const entries = hook
    .split('\n')
    .filter((line: string) => line.includes('LangString LauncherFolderChanged'))
  expect(entries).toHaveLength(3)
  for (const language of ['LANG_ENGLISH', 'LANG_SIMPCHINESE', 'LANG_TRADCHINESE']) {
    expect(
      entries.some((line: string) => line.includes(`{${language}}`)),
      `the hook has no copy for ${language}`,
    ).toBe(true)
  }
  // A two-way $LANGUAGE branch would send every other installer language to English.
  expect(hook).not.toContain('$LANGUAGE =')
})
