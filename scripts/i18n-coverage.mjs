#!/usr/bin/env node
// Reports how much of the English source each catalog covers, and fails when a
// language drops below the percentage recorded in src/i18n/coverage-baseline.json.
// `--update` rewrites that baseline, which is the deliberate way to accept a drop.
import { readFileSync, writeFileSync } from 'node:fs'
import { dirname, join } from 'node:path'
import { fileURLToPath } from 'node:url'

const root = join(dirname(fileURLToPath(import.meta.url)), '..')
const catalogs = { 'zh-Hans': 'src/i18n/zh-Hans.ts', 'zh-Hant': 'src/i18n/zh-Hant.ts' }
const baselinePath = join(root, 'src/i18n/coverage-baseline.json')

// Only the message object counts: the file also holds translator notes, whose keys are
// formatted the same way and must not inflate or dilute the percentage.
const keysOf = (path) => {
  const source = readFileSync(path, 'utf8')
  const block = /export const messages[^=]*= \{([\s\S]*?)\n\}/.exec(source)?.[1] ?? ''
  return [...block.matchAll(/^ {2}'([a-z][A-Za-z]*\.[A-Za-z]+)':/gm)].map((match) => match[1])
}

const source = keysOf(join(root, 'src/i18n/en.ts'))
const report = {}
for (const [language, path] of Object.entries(catalogs)) {
  const keys = new Set(keysOf(join(root, path)))
  const missing = source.filter((key) => !keys.has(key))
  report[language] = {
    total: source.length,
    translated: keys.size,
    percent: Math.round((keys.size / source.length) * 100),
    missing,
  }
}

for (const [language, entry] of Object.entries(report)) {
  console.log(`${language}: ${entry.translated}/${entry.total} keys (${entry.percent}%)`)
  for (const key of entry.missing) console.log(`  missing ${key}`)
}

if (process.argv.includes('--update')) {
  const next = Object.fromEntries(
    Object.entries(report).map(([language, entry]) => [language, entry.translated]),
  )
  writeFileSync(baselinePath, `${JSON.stringify(next, null, 2)}\n`)
  console.log(`baseline written to ${baselinePath} (translated key counts)`)
} else {
  const baseline = JSON.parse(readFileSync(baselinePath, 'utf8'))
  const dropped = Object.entries(report).filter(([language, entry]) => {
    return entry.translated < (baseline[language] ?? 0)
  })
  if (dropped.length) {
    console.error(
      `coverage dropped below the baseline: ${dropped
        .map(([language, entry]) => `${language} ${entry.translated} < ${baseline[language]} keys`)
        .join(', ')}`,
    )
    process.exit(1)
  }
}
