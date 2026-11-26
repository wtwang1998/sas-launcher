import { expect, it } from 'vitest'
import {
  checkForUpdate,
  compareVersions,
  newestRelease,
  parseReleases,
  type Release,
  type UpdateCheck,
} from './update'

/** A fetch that answers once with a canned status and body. */
const reply = (status: number, body: unknown, headers: Record<string, string> = {}) => {
  return async () =>
    ({
      ok: status >= 200 && status < 300,
      status,
      headers: new Headers(headers),
      json: async () => body,
    }) as unknown as Response
}

const unreachable = async (): Promise<Response> => {
  throw new TypeError('Failed to fetch')
}

const timedOut = async (): Promise<Response> => {
  throw new DOMException('The operation was aborted.', 'AbortError')
}

it('a_ten_release_sorts_above_the_nine_before_it', () => {
  const cases: { left: string; right: string; order: -1 | 0 | 1 }[] = [
    { left: 'v0.3.10', right: 'v0.3.9', order: 1 },
    { left: 'v0.3.0', right: 'v0.2.9', order: 1 },
    { left: 'v0.3.0', right: '0.3.0', order: 0 },
    { left: 'v1.0.0-rc.1', right: 'v1.0.0', order: -1 },
    { left: 'v1.0.0', right: 'v1.0.0-rc.1', order: 1 },
    { left: 'v0.4.0-rc.1', right: 'v0.3.0', order: 1 },
    { left: 'v0.3.0-rc.2', right: 'v0.3.0-rc.10', order: -1 },
  ]

  for (const { left, right, order } of cases) {
    expect(Math.sign(compareVersions(left, right)), `${left} vs ${right}`).toBe(order)
    expect(Math.sign(compareVersions(right, left)), `${right} vs ${left}`).toBe(
      order === 0 ? 0 : -order,
    )
  }
})

it('a_draft_is_not_a_release_anyone_can_reach', () => {
  const releases = parseReleases([
    { tag_name: 'v0.5.0', draft: true, prerelease: true },
    { tag_name: 'v0.4.0', draft: false, prerelease: true },
    { tag_name: 'v0.3.9', prerelease: false },
    { name: 'no tag at all' },
    'not even an object',
  ])

  expect(releases).toEqual<Release[]>([
    { tag: 'v0.4.0', prerelease: true },
    { tag: 'v0.3.9', prerelease: false },
  ])
})

it('a_body_that_is_not_a_list_cannot_be_read', () => {
  expect(parseReleases({ message: 'API rate limit exceeded' })).toBeNull()
  expect(parseReleases(null)).toBeNull()
  expect(parseReleases([])).toEqual([])
})

it('the_newest_release_has_to_be_above_the_one_running', () => {
  const releases: Release[] = [
    { tag: 'v0.2.0', prerelease: true },
    { tag: 'v0.4.0-rc.2', prerelease: true },
    { tag: 'v0.4.0-rc.10', prerelease: true },
    { tag: 'v0.3.0', prerelease: true },
  ]

  expect(newestRelease(releases, '0.3.0')).toEqual<Release>({
    tag: 'v0.4.0-rc.10',
    prerelease: true,
  })
  expect(newestRelease(releases, '0.4.0')).toBeNull()
  expect(newestRelease([], '0.3.0')).toBeNull()
})

it('the_check_reports_what_it_found_and_why_it_stopped', async () => {
  const cases: { name: string; fetch: () => Promise<Response>; expected: UpdateCheck }[] = [
    {
      name: 'newer',
      fetch: reply(200, [{ tag_name: 'v0.4.0', draft: false, prerelease: true }]),
      expected: { kind: 'available', tag: 'v0.4.0', prerelease: true },
    },
    {
      name: 'same',
      fetch: reply(200, [{ tag_name: 'v0.3.0', draft: false, prerelease: true }]),
      expected: { kind: 'latest' },
    },
    {
      name: 'rate limited',
      fetch: reply(403, { message: 'API rate limit exceeded' }, { 'x-ratelimit-remaining': '0' }),
      expected: { kind: 'failed', reason: 'rateLimited' },
    },
    {
      name: 'forbidden for another reason',
      fetch: reply(403, { message: 'nope' }, { 'x-ratelimit-remaining': '7' }),
      expected: { kind: 'failed', reason: 'unreadable' },
    },
    {
      name: 'an html login page',
      fetch: async () =>
        ({ ok: true, status: 200, headers: new Headers(), json: async () => '<html>' }) as never,
      expected: { kind: 'failed', reason: 'unreadable' },
    },
    { name: 'unreachable', fetch: unreachable, expected: { kind: 'failed', reason: 'offline' } },
    { name: 'timed out', fetch: timedOut, expected: { kind: 'failed', reason: 'timeout' } },
  ]

  for (const { name, fetch, expected } of cases) {
    expect(await checkForUpdate('0.3.0', fetch), name).toEqual(expected)
  }
})
