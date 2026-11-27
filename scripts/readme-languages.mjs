const LANGUAGES = [
  { tag: 'en', file: 'README.md', label: 'English' },
  { tag: 'zh-Hans', file: 'README.zh-CN.md', label: '简体中文' },
  { tag: 'zh-Hant', file: 'README.zh-Hant.md', label: '繁體中文' },
  { tag: 'ja', file: 'README.ja.md', label: '日本語' },
  { tag: 'ko', file: 'README.ko.md', label: '한국어' },
  { tag: 'de', file: 'README.de.md', label: 'Deutsch' },
  { tag: 'fr', file: 'README.fr.md', label: 'Français' },
  { tag: 'es', file: 'README.es.md', label: 'Español' },
]

const newlines = /\r\n/g

function readmeHrefs(line) {
  return [...line.matchAll(/\]\(\.\/(README[^)]*\.md)\)/g)].map((match) => match[1])
}

/**
 * Picks the language switcher out of a readme. It is the one line that links
 * to every other readme, so a prose sentence mentioning one of them cannot be
 * mistaken for it.
 */
function parseSwitcher(markdown, expected = LANGUAGES.length - 1) {
  const lines = markdown.split(/\r?\n/)
  const line = lines.find((candidate) => readmeHrefs(candidate).length >= expected)
  if (!line) return null
  return line.split('|').map((part) => {
    const entry = part.trim()
    const current = /^\*\*(.+)\*\*$/.exec(entry)
    if (current) return { label: current[1], href: null }
    const link = /^\[(.+)\]\(\.\/(README[^)]*\.md)\)$/.exec(entry)
    if (!link) return { label: entry, href: null, malformed: true }
    return { label: link[1], href: link[2] }
  })
}

function headingCounts(markdown) {
  const lines = markdown.split(/\r?\n/)
  return {
    h2: lines.filter((line) => /^## /.test(line)).length,
    h3: lines.filter((line) => /^### /.test(line)).length,
  }
}

function fences(markdown) {
  return [...markdown.matchAll(/^```[^\n]*\n([\s\S]*?)^```$/gm)].map((match) =>
    match[1].replace(newlines, '\n'),
  )
}

function relativeLinks(markdown) {
  return [...markdown.matchAll(/\[[^\]]*\]\((?!https?:|#|mailto:)([^)]+)\)/g)]
    .map((match) => match[1])
    .filter((href) => !href.startsWith('http'))
}

/**
 * Compares every translation against the English readme: same structure, same
 * code, same switcher. The prose is free; the skeleton is not.
 */
function checkReadmes(files) {
  const problems = []
  const english = files['README.md']
  if (english === undefined) return ['README.md is missing']
  const reference = { headings: headingCounts(english), fences: fences(english) }

  for (const { tag, file, label } of LANGUAGES) {
    const markdown = files[file]
    if (markdown === undefined) {
      problems.push(`${file} is missing`)
      continue
    }

    const headings = headingCounts(markdown)
    if (headings.h2 !== reference.headings.h2) {
      problems.push(
        `${file} has ${headings.h2} H2 headings, README.md has ${reference.headings.h2}`,
      )
    }
    if (headings.h3 !== reference.headings.h3) {
      problems.push(
        `${file} has ${headings.h3} H3 headings, README.md has ${reference.headings.h3}`,
      )
    }

    const code = fences(markdown)
    if (code.length !== reference.fences.length) {
      problems.push(
        `${file} has ${code.length} code blocks, README.md has ${reference.fences.length}`,
      )
    }
    code.forEach((block, index) => {
      if (block !== reference.fences[index]) {
        problems.push(`${file} code block ${index + 1} differs from README.md`)
      }
    })

    const switcher = parseSwitcher(markdown)
    if (!switcher) {
      problems.push(`${file} has no language switcher`)
    } else if (switcher.length !== LANGUAGES.length) {
      problems.push(`${file} switcher lists ${switcher.length} languages, not ${LANGUAGES.length}`)
    } else {
      LANGUAGES.forEach((entry, index) => {
        const item = switcher[index]
        if (item.malformed) {
          problems.push(`${file} switcher entry ${index + 1} is not a link or a bold label`)
          return
        }
        if (item.label !== entry.label) {
          problems.push(
            `${file} switcher entry ${index + 1} reads ${item.label}, not ${entry.label}`,
          )
        }
        const current = entry.file === file
        if (current && item.href !== null) {
          problems.push(`${file} links to itself in the switcher`)
        }
        if (!current && item.href !== entry.file) {
          problems.push(
            `${file} switcher entry ${index + 1} links to ${item.href}, not ${entry.file}`,
          )
        }
      })
      const current = switcher.filter((item) => item.href === null && !item.malformed)
      if (current.length !== 1) {
        problems.push(
          `${file} switcher marks ${current.length} languages as current, not 1 (${label}?)`,
        )
      }
    }
  }

  return problems
}

export { checkReadmes, fences, headingCounts, LANGUAGES, parseSwitcher, relativeLinks }
