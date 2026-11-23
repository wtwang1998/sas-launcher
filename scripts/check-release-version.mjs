// Fails a release run when the pushed tag and the three manifests disagree.
// The tag is the only input that cannot be edited after the release exists, so
// nothing here guesses a winner: any mismatch stops the build.
import { readFileSync } from 'node:fs'
import { dirname, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), '..')

const SOURCES = [
  ['package.json', 'packageJson'],
  ['src-tauri/Cargo.toml', 'cargo'],
  ['src-tauri/tauri.conf.json', 'tauriConf'],
]

export function parseReleaseTag(tag) {
  const match = /^v(\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?)$/.exec(tag ?? '')
  return match ? match[1] : null
}

export function readCargoVersion(manifest) {
  let inPackage = false
  for (const raw of manifest.split(/\r?\n/)) {
    const line = raw.trim()
    if (line.startsWith('[')) {
      inPackage = line === '[package]'
      continue
    }
    if (!inPackage) continue
    const match = /^version\s*=\s*"([^"]+)"/.exec(line)
    if (match) return match[1]
  }
  return null
}

export function checkReleaseVersion(tag, versions) {
  const version = parseReleaseTag(tag)
  if (!version) {
    return {
      ok: false,
      version: null,
      problems: [`tag must look like v1.2.3, got ${JSON.stringify(tag ?? '')}`],
    }
  }

  const problems = []
  for (const [file, key] of SOURCES) {
    const actual = versions[key]
    if (actual !== version) {
      problems.push(`${file} is ${actual ?? 'missing'}, tag says ${version} (${tag})`)
    }
  }
  return { ok: problems.length === 0, version, problems }
}

function readVersions() {
  const read = (path) => readFileSync(resolve(ROOT, path), 'utf8')
  return {
    packageJson: JSON.parse(read('package.json')).version,
    cargo: readCargoVersion(read('src-tauri/Cargo.toml')),
    tauriConf: JSON.parse(read('src-tauri/tauri.conf.json')).version,
  }
}

function main() {
  const tag = process.argv[2]
  const result = checkReleaseVersion(tag, readVersions())
  if (result.ok) {
    console.log(
      `release ${tag}: package.json, Cargo.toml and tauri.conf.json all say ${result.version}`,
    )
    return
  }
  for (const problem of result.problems) console.error(`error: ${problem}`)
  process.exitCode = 1
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  main()
}
