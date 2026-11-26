// Fails a release run when the pushed tag and every version source disagree.
// The tag is the only input that cannot be edited after the release exists, so
// nothing here guesses a winner: any mismatch stops the build.
import { readFileSync } from 'node:fs'
import { dirname, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), '..')

// v0.3.0 failed CI because the bump missed package-lock.json: the manifests
// agreed with the tag while the lock still named the previous version.
const SOURCES = [
  ['packageJson', 'package.json'],
  ['lockRoot', 'package-lock.json (root)'],
  ['lockPackage', 'package-lock.json (packages[""])'],
  ['cargo', 'src-tauri/Cargo.toml'],
  ['cargoLock', 'src-tauri/Cargo.lock (sas-launcher)'],
  ['tauriConf', 'src-tauri/tauri.conf.json'],
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

export function readCargoLockVersion(lock) {
  let packageName = null
  for (const raw of lock.split(/\r?\n/)) {
    const line = raw.trim()
    if (line.startsWith('[')) {
      packageName = null
      continue
    }
    const name = /^name\s*=\s*"([^"]+)"/.exec(line)
    if (name) {
      packageName = name[1]
      continue
    }
    const version = /^version\s*=\s*"([^"]+)"/.exec(line)
    if (version && packageName === 'sas-launcher') return version[1]
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
  for (const [key, where] of SOURCES) {
    const actual = versions[key]
    if (actual !== version) {
      problems.push(`${where} is ${actual ?? 'missing'}, tag says ${version} (${tag})`)
    }
  }
  return { ok: problems.length === 0, version, problems }
}

function readVersions() {
  const read = (path) => readFileSync(resolve(ROOT, path), 'utf8')
  const lock = JSON.parse(read('package-lock.json'))
  return {
    packageJson: JSON.parse(read('package.json')).version,
    lockRoot: lock.version,
    lockPackage: lock.packages?.['']?.version,
    cargo: readCargoVersion(read('src-tauri/Cargo.toml')),
    cargoLock: readCargoLockVersion(read('src-tauri/Cargo.lock')),
    tauriConf: JSON.parse(read('src-tauri/tauri.conf.json')).version,
  }
}

function main() {
  const tag = process.argv[2]
  const result = checkReleaseVersion(tag, readVersions())
  if (result.ok) {
    console.log(
      `release ${tag}: package.json, package-lock.json, Cargo.toml, Cargo.lock and tauri.conf.json all say ${result.version}`,
    )
    return
  }
  for (const problem of result.problems) console.error(`error: ${problem}`)
  process.exitCode = 1
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  main()
}
