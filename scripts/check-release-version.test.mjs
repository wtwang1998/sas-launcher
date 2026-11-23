import { describe, expect, it } from 'vitest'
import { checkReleaseVersion, parseReleaseTag, readCargoVersion } from './check-release-version.mjs'

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

describe('checkReleaseVersion', () => {
  it('accepts a tag that matches all three manifests', () => {
    expect(
      checkReleaseVersion('v0.2.0', { packageJson: '0.2.0', cargo: '0.2.0', tauriConf: '0.2.0' }),
    ).toEqual({
      ok: true,
      version: '0.2.0',
      problems: [],
    })
  })

  it('accepts a prerelease tag when every manifest carries the same prerelease', () => {
    const result = checkReleaseVersion('v1.0.0-rc.1', {
      packageJson: '1.0.0-rc.1',
      cargo: '1.0.0-rc.1',
      tauriConf: '1.0.0-rc.1',
    })
    expect(result.ok).toBe(true)
    expect(result.version).toBe('1.0.0-rc.1')
  })

  it('reports every manifest that disagrees with the tag', () => {
    const result = checkReleaseVersion('v0.3.0', {
      packageJson: '0.3.0',
      cargo: '0.2.0',
      tauriConf: '0.2.1',
    })
    expect(result.ok).toBe(false)
    expect(result.problems).toHaveLength(2)
    expect(result.problems.join('\n')).toContain('0.2.0')
    expect(result.problems.join('\n')).toContain('0.2.1')
  })

  it('reports a tag without the v prefix instead of guessing', () => {
    const result = checkReleaseVersion('0.2.0', {
      packageJson: '0.2.0',
      cargo: '0.2.0',
      tauriConf: '0.2.0',
    })
    expect(result.ok).toBe(false)
    expect(result.version).toBeNull()
    expect(result.problems.join('\n')).toContain('v1.2.3')
  })

  it('reports a manifest that disagrees in the other direction', () => {
    const result = checkReleaseVersion('v0.2.0', {
      packageJson: '0.2.0',
      cargo: '0.2.0',
      tauriConf: '0.2.1',
    })
    expect(result.ok).toBe(false)
    expect(result.problems.join('\n')).toContain('0.2.1')
    expect(result.problems.join('\n')).not.toContain('0.2.0 is')
  })
})
