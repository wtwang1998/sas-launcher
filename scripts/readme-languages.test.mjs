import { existsSync, readFileSync } from 'node:fs'
import { describe, expect, it } from 'vitest'
import {
  checkReadmes,
  fences,
  headingCounts,
  LANGUAGES,
  parseSwitcher,
  relativeLinks,
} from './readme-languages.mjs'

const switcher = [
  '**English** | [简体中文](./README.zh-CN.md) | [繁體中文](./README.zh-Hant.md)',
  '[日本語](./README.ja.md) | [한국어](./README.ko.md) | [Deutsch](./README.de.md)',
  '[Français](./README.fr.md) | [Español](./README.es.md)',
].join(' | ')

function readme(overrides = {}) {
  return {
    switcher,
    body: '## Why\n\ntext\n\n```sas\n%put hi;\n```\n',
    ...overrides,
  }
}

function document({ switcher: line = switcher, body = '## Why\n\ntext\n' } = {}) {
  return `# SAS Launcher\n\n${line}\n\n${body}`
}

describe('parseSwitcher', () => {
  it('reads the bolded entry as the current language and the rest as links', () => {
    const entries = parseSwitcher(document())
    expect(entries).toHaveLength(8)
    expect(entries[0]).toEqual({ label: 'English', href: null })
    expect(entries[1]).toEqual({ label: '简体中文', href: 'README.zh-CN.md' })
  })

  it('ignores prose that links to a single readme', () => {
    const markdown = `${document()}\nSee [English](./README.md) for more.\n`
    expect(parseSwitcher(markdown)).toHaveLength(8)
  })

  it('returns null when the line is missing', () => {
    expect(parseSwitcher('# SAS Launcher\n\nNo switcher here.\n')).toBeNull()
  })
})

describe('headingCounts', () => {
  it('counts H2 and H3 separately', () => {
    expect(headingCounts('## A\n### B\n## C\n#### D\n')).toEqual({ h2: 2, h3: 1 })
  })
})

describe('fences', () => {
  it('keeps the content of each block and drops the language tag', () => {
    expect(fences('```sas\n%put hi;\n```\n\ntext\n\n```powershell\nnpm ci\n```\n')).toEqual([
      '%put hi;\n',
      'npm ci\n',
    ])
  })
})

describe('relativeLinks', () => {
  it('keeps local targets and drops web, anchor and mail links', () => {
    const markdown =
      '[a](docs/a.png) [b](https://x.test) [c](#top) [d](mailto:x@y) ![e](docs/e.png)\n'
    expect(relativeLinks(markdown)).toEqual(['docs/a.png', 'docs/e.png'])
  })
})

describe('checkReadmes', () => {
  const complete = () => {
    const files = {}
    for (const language of LANGUAGES) {
      const label = language.label === 'English' ? `**English**` : `**${language.label}**`
      const line = LANGUAGES.map((entry) =>
        entry.file === language.file ? label : `[${entry.label}](./${entry.file})`,
      ).join(' | ')
      files[language.file] = `# SAS Launcher\n\n${line}\n\n${readme().body}`
    }
    return files
  }

  it('accepts a complete set', () => {
    expect(checkReadmes(complete())).toEqual([])
  })

  it('reports a missing translation', () => {
    const files = complete()
    delete files['README.ja.md']
    expect(checkReadmes(files)).toContain('README.ja.md is missing')
  })

  it('reports a translation with a different section count', () => {
    const files = complete()
    files['README.de.md'] = files['README.de.md'].replace('## Why\n', '## Warum\n## Noch eins\n')
    expect(checkReadmes(files).join('\n')).toContain('README.de.md has 2 H2 headings')
  })

  it('reports a translation whose code block drifted', () => {
    const files = complete()
    files['README.fr.md'] = files['README.fr.md'].replace('%put hi;', '%put bonjour;')
    expect(checkReadmes(files).join('\n')).toContain('README.fr.md code block 1 differs')
  })

  it('reports a switcher that forgets a language', () => {
    const files = complete()
    files['README.es.md'] = files['README.es.md'].replace(' | [Deutsch](./README.de.md)', '')
    expect(checkReadmes(files).join('\n')).toContain('README.es.md has no language switcher')
  })

  it('reports a translation that links to itself', () => {
    const files = complete()
    files['README.ko.md'] = files['README.ko.md'].replace(
      '**한국어** | [Deutsch]',
      '[한국어](./README.ko.md) | [Deutsch]',
    )
    expect(checkReadmes(files).join('\n')).toContain('README.ko.md links to itself')
  })
})

describe('the readmes in this repository', () => {
  const files = Object.fromEntries(
    LANGUAGES.map((language) => [language.file, readFileSync(language.file, 'utf8')]),
  )

  it('keep one structure, one switcher and one set of code blocks', () => {
    expect(checkReadmes(files)).toEqual([])
  })

  it('only link to files that exist', () => {
    const missing = []
    for (const language of LANGUAGES) {
      for (const href of relativeLinks(files[language.file])) {
        if (!existsSync(href)) missing.push(`${language.file}: ${href}`)
      }
    }
    expect(missing).toEqual([])
  })
})
