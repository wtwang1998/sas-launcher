/** One published release, as far as the about page cares. */
export type Release = { tag: string; prerelease: boolean }

/** Where a check landed, including the reasons it can stop early. */
export type UpdateCheck =
  | { kind: 'idle' }
  | { kind: 'checking' }
  | { kind: 'latest' }
  | { kind: 'available'; tag: string; prerelease: boolean }
  | { kind: 'failed'; reason: 'timeout' | 'offline' | 'rateLimited' | 'unreadable' }

export type Fetch = (url: string, init?: RequestInit) => Promise<Response>

const RELEASES = 'https://api.github.com/repos/wtwang1998/sas-launcher/releases?per_page=10'
const TIMEOUT_MS = 30_000

/** Splits `v1.2.3-rc.1` into its numbers and its pre-release tail. */
function splitVersion(version: string) {
  const bare = version.trim().replace(/^v/i, '')
  const dash = bare.indexOf('-')
  const core = dash === -1 ? bare : bare.slice(0, dash)
  const pre = dash === -1 ? '' : bare.slice(dash + 1)
  const numbers = core.split('.').map((part) => Number.parseInt(part, 10) || 0)
  while (numbers.length < 3) numbers.push(0)
  return { numbers, pre }
}

function comparePre(left: string, right: string) {
  const a = left.split('.')
  const b = right.split('.')
  for (let index = 0; index < Math.max(a.length, b.length); index += 1) {
    if (a[index] === undefined) return -1
    if (b[index] === undefined) return 1
    if (/^\d+$/.test(a[index]) && /^\d+$/.test(b[index])) {
      const difference = Number(a[index]) - Number(b[index])
      if (difference !== 0) return difference
    } else if (a[index] !== b[index]) {
      return a[index] < b[index] ? -1 : 1
    }
  }
  return 0
}

/** `v0.3.10` sorts above `v0.3.9`, and a pre-release below its own release. */
export function compareVersions(left: string, right: string) {
  const a = splitVersion(left)
  const b = splitVersion(right)
  for (let index = 0; index < 3; index += 1) {
    if (a.numbers[index] !== b.numbers[index]) return a.numbers[index] - b.numbers[index]
  }
  if (a.pre === b.pre) return 0
  if (!a.pre) return 1
  if (!b.pre) return -1
  return comparePre(a.pre, b.pre)
}

/** The releases the API lists, minus the drafts only a maintainer can see. */
export function parseReleases(body: unknown): Release[] | null {
  if (!Array.isArray(body)) return null
  const releases: Release[] = []
  for (const entry of body) {
    if (!entry || typeof entry !== 'object') continue
    const record = entry as Record<string, unknown>
    if (record.draft === true || typeof record.tag_name !== 'string') continue
    releases.push({ tag: record.tag_name, prerelease: record.prerelease === true })
  }
  return releases
}

/** The highest listed release that is newer than the one running. */
export function newestRelease(releases: Release[], current: string): Release | null {
  let newest: Release | null = null
  for (const release of releases) {
    if (compareVersions(release.tag, current) <= 0) continue
    if (!newest || compareVersions(release.tag, newest.tag) > 0) newest = release
  }
  return newest
}

/**
 * One unauthenticated request to the releases list. Its rate limit is sixty an
 * hour per origin address, so this only ever runs on a click, never on a timer.
 */
export async function checkForUpdate(
  current: string,
  fetcher: Fetch = (url, init) => fetch(url, init),
  timeout = TIMEOUT_MS,
): Promise<UpdateCheck> {
  const abort = new AbortController()
  const timer = setTimeout(() => abort.abort(), timeout)
  try {
    const response = await fetcher(RELEASES, {
      signal: abort.signal,
      headers: { accept: 'application/vnd.github+json' },
    })
    if (response.status === 403 && response.headers.get('x-ratelimit-remaining') === '0') {
      return { kind: 'failed', reason: 'rateLimited' }
    }
    if (!response.ok) return { kind: 'failed', reason: 'unreadable' }
    let body: unknown
    try {
      body = await response.json()
    } catch {
      return { kind: 'failed', reason: 'unreadable' }
    }
    const releases = parseReleases(body)
    if (!releases) return { kind: 'failed', reason: 'unreadable' }
    const newest = newestRelease(releases, current)
    return newest ? { kind: 'available', ...newest } : { kind: 'latest' }
  } catch (error) {
    const name =
      error && typeof error === 'object' && 'name' in error ? String(error.name) : 'unknown'
    return { kind: 'failed', reason: name === 'AbortError' ? 'timeout' : 'offline' }
  } finally {
    clearTimeout(timer)
  }
}
