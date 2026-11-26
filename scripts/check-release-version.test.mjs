import { describe, expect, it } from 'vitest'
import {
  checkReleaseVersion,
  parseReleaseTag,
  readCargoLockVersion,
  readCargoVersion,
} from './check-release-version.mjs'

function sources(overrides = {}) {
  return {
    packageJson: '0.2.0',
    lockRoot: '0.2.0',
    lockPackage: '0.2.0',
    cargo: '0.2.0',
    cargoLock: '0.2.0',
    tauriConf: '0.2.0',
    ...overrides,
  }
}

describe('parseReleaseTag', () => {
  it.each([
    ['v0.2.0', '0.2.0'],
    ['v1.0.0-rc.1', '1.0.0-rc.1'],
    ['v10.20.30', '10.20.30'],
    ['0.2.0', null],
    ['V0.2.0', null],
    ['v0.2', null],
    ['v0.2.0.1', null],
    ['release-v0.2.0', null],
    ['', null],
  ])('%s -> %s', (tag, expected) => {
    expect(parseReleaseTag(tag)).toBe(expected)
  })
})

describe('readCargoVersion', () => {
  it('reads the version of the package section', () => {
    const manifest = [
      '[package]',
      'name = "sas-launcher"',
      'version = "0.2.0"',
      '',
      '[dependencies]',
      '',
    ].join('\n')
    expect(readCargoVersion(manifest)).toBe('0.2.0')
  })

  it('ignores a version that belongs to a dependency table', () => {
    const manifest = [
      '[package]',
      'name = "sas-launcher"',
      '',
      '[dependencies.serde]',
      'version = "1"',
      '',
    ].join('\n')
    expect(readCargoVersion(manifest)).toBeNull()
  })

  it('returns null when the package section is missing', () => {
    expect(readCargoVersion('[dependencies]\nserde = "1"\n')).toBeNull()
  })
})

describe('readCargoLockVersion', () => {
  it('reads the version of the sas-launcher package', () => {
    const lock = [
      'version = 4',
      '',
      '[[package]]',
      'name = "sas-launcher"',
      'version = "0.2.0"',
      'dependencies = []',
      '',
    ].join('\n')
    expect(readCargoLockVersion(lock)).toBe('0.2.0')
  })

  it('does not take the version of another package', () => {
    const lock = [
      '[[package]]',
      'name = "serde"',
      'version = "1.0.0"',
      '',
      '[[package]]',
      'name = "sas-launcher"',
      'version = "0.2.0"',
      '',
    ].join('\n')
    expect(readCargoLockVersion(lock)).toBe('0.2.0')
  })

  it('returns null when the package is missing', () => {
    expect(readCargoLockVersion('[[package]]\nname = "serde"\nversion = "1"\n')).toBeNull()
  })
})

describe('checkReleaseVersion', () => {
  it('accepts a tag that matches every source', () => {
    expect(checkReleaseVersion('v0.2.0', sources())).toEqual({
      ok: true,
      version: '0.2.0',
      problems: [],
    })
  })

  it('accepts a prerelease tag when every source carries the same prerelease', () => {
    const result = checkReleaseVersion(
      'v1.0.0-rc.1',
      sources({
        packageJson: '1.0.0-rc.1',
        lockRoot: '1.0.0-rc.1',
        lockPackage: '1.0.0-rc.1',
        cargo: '1.0.0-rc.1',
        cargoLock: '1.0.0-rc.1',
        tauriConf: '1.0.0-rc.1',
      }),
    )
    expect(result.ok).toBe(true)
    expect(result.version).toBe('1.0.0-rc.1')
  })

  it.each([
    ['packageJson', 'package.json'],
    ['lockRoot', 'package-lock.json (root)'],
    ['lockPackage', 'package-lock.json (packages[""])'],
    ['cargo', 'src-tauri/Cargo.toml'],
    ['cargoLock', 'src-tauri/Cargo.lock (sas-launcher)'],
    ['tauriConf', 'src-tauri/tauri.conf.json'],
  ])('reports %s when it disagrees with the tag', (key, where) => {
    const result = checkReleaseVersion('v0.2.0', sources({ [key]: '0.1.0' }))
    expect(result.ok).toBe(false)
    expect(result.problems).toHaveLength(1)
    expect(result.problems[0]).toContain(where)
    expect(result.problems[0]).toContain('0.1.0')
  })

  it('reports a source that is missing instead of passing it', () => {
    const result = checkReleaseVersion('v0.2.0', sources({ cargoLock: undefined }))
    expect(result.ok).toBe(false)
    expect(result.problems.join('\n')).toContain('missing')
  })

  it('reports a tag without the v prefix instead of guessing', () => {
    const result = checkReleaseVersion('0.2.0', sources())
    expect(result.ok).toBe(false)
    expect(result.version).toBeNull()
    expect(result.problems.join('\n')).toContain('v1.2.3')
  })
})
