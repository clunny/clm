// The repository's star count, read once per build (`github-link`).

/** `1234` as `1.2k`, `1950` as `2.0k`, and a count under 1000 as it is. */
export function formatStars(n: number): string {
  return n < 1000 ? String(n) : `${(Math.round(n / 100) / 10).toFixed(1)}k`
}

/** The count to show: `GITHUB_STARS` when the environment sets it, else GitHub's `stargazers_count`
 * for `repo`; undefined, with a warning, when it can't be read, so a build never fails on it. */
export async function readStars(repo: string): Promise<string | undefined> {
  const given = process.env.GITHUB_STARS
  if (given !== undefined && given !== '') {
    const n = Number(given)
    if (Number.isFinite(n)) return formatStars(n)
    console.warn(`warning: GITHUB_STARS=${given} isn't a number; the GitHub link shows without a count`)
    return undefined
  }
  const api = process.env.GITHUB_API || 'https://api.github.com'
  try {
    const headers: Record<string, string> = { Accept: 'application/vnd.github+json' }
    if (process.env.GITHUB_TOKEN) headers.Authorization = `Bearer ${process.env.GITHUB_TOKEN}`
    const res = await fetch(`${api}/repos/${repo}`, { headers, signal: AbortSignal.timeout(10_000) })
    if (!res.ok) throw new Error(`HTTP ${res.status}`)
    const count = ((await res.json()) as { stargazers_count?: unknown }).stargazers_count
    if (typeof count !== 'number') throw new Error('no stargazers_count in the answer')
    return formatStars(count)
  } catch (e) {
    console.warn(`warning: the star count of ${repo} couldn't be read (${(e as Error).message}); the GitHub link shows without one`)
    return undefined
  }
}
