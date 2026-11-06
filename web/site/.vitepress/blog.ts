import { existsSync, readFileSync, readdirSync } from 'node:fs'
import { join, posix, resolve } from 'node:path'
import { parse } from 'yaml'
import { repoLink } from './guide'

// The directory the posts are read from (`BLOG_DIR` points tests at a fixture, as `GUIDE_DIR`
// does for the guide); a missing or empty one builds a blog with no posts.
const blogDir = () =>
  process.env.BLOG_DIR ? resolve(process.env.BLOG_DIR) : resolve(import.meta.dirname, '../blog')

export interface Post {
  slug: string
  file: string
  title: string
  summary: string
  date: string // YYYY-MM-DD
  tags: string[]
  part?: number // series_part
  draft: boolean
  body: string // the file's text without its frontmatter
  minutes: number // reading time
}

const MONTHS = ['Jan', 'Feb', 'Mar', 'Apr', 'May', 'Jun', 'Jul', 'Aug', 'Sep', 'Oct', 'Nov', 'Dec']

/** `2026-11-05` as `5 Nov 2026`. */
export function dateLabel(date: string): string {
  const [y, m, d] = date.split('-').map(Number)
  return `${d} ${MONTHS[m - 1]} ${y}`
}

/** A reading time in minutes: the body's whitespace-separated tokens over 230, rounded up, at
 * least one (`blog-index`). */
export const readingMinutes = (body: string): number =>
  Math.max(1, Math.ceil(body.split(/\s+/).filter(Boolean).length / 230))

const isDate = (s: unknown): s is string =>
  typeof s === 'string' &&
  /^\d{4}-\d{2}-\d{2}$/.test(s) &&
  new Date(`${s}T00:00:00Z`).toISOString().slice(0, 10) === s

/** Every post of the blog, drafts included, each checked; throws, naming the file, on a bad one
 * (`blog-drafts-left-out`). */
export function loadPosts(): Post[] {
  const dir = blogDir()
  if (!existsSync(dir)) return []
  const cached = loaded.get(dir)
  if (cached) return cached
  const posts = readParsed(dir)
  loaded.set(dir, posts)
  return posts
}

// A build reads its posts once: every page's head asks for them.
const loaded = new Map<string, Post[]>()

function readParsed(dir: string): Post[] {
  const posts = readdirSync(dir)
    .filter((f) => f.endsWith('.md'))
    .sort()
    .map((f): Post => {
      const file = join(dir, f)
      const slug = f.slice(0, -3)
      const fail = (why: string): never => {
        throw new Error(`${file}: ${why}`)
      }
      const text = readFileSync(file, 'utf8')
      const m = /^---\r?\n([\s\S]*?)\r?\n---[ \t]*(?:\r?\n|$)/.exec(text)
      if (!m) return fail('the post has no frontmatter')
      let data: Record<string, unknown>
      try {
        data = (parse(m[1]) ?? {}) as Record<string, unknown>
      } catch (e) {
        return fail(`the frontmatter isn't valid YAML (${(e as Error).message})`)
      }
      if (typeof data !== 'object' || Array.isArray(data)) return fail("the frontmatter isn't a mapping")
      if (slug === 'index' || slug === 'feed') return fail(`the slug ${slug} names the blog's own page or feed`)
      for (const key of ['title', 'summary', 'date']) {
        if (typeof data[key] !== 'string' || (data[key] as string).trim() === '') {
          fail(`the post has no ${key}`)
        }
      }
      if (!isDate(data.date)) fail(`date must be a real calendar date written YYYY-MM-DD, not ${String(data.date)}`)
      if (data.draft !== undefined && typeof data.draft !== 'boolean') fail('draft must be true or false')
      const tags = data.tags ?? []
      if (!Array.isArray(tags) || tags.some((t) => typeof t !== 'string')) fail('tags must be a list of words')
      const part = data.series_part
      if (part !== undefined && !(typeof part === 'number' && Number.isInteger(part) && part > 0)) {
        fail('series_part must be a positive integer')
      }
      const body = text.slice(m[0].length)
      return {
        slug,
        file,
        title: (data.title as string).trim(),
        summary: (data.summary as string).trim(),
        date: data.date as string,
        tags: tags as string[],
        part: part as number | undefined,
        draft: data.draft !== false,
        body,
        minutes: readingMinutes(body),
      }
    })
  const seen = new Map<number, Post>()
  for (const p of posts.filter((p) => !p.draft && p.part !== undefined)) {
    const other = seen.get(p.part!)
    if (other) throw new Error(`${p.file}: series_part ${p.part} is also ${other.file}'s`)
    seen.set(p.part!, p)
  }
  return posts
}

/** The published posts, newest first by date, then highest series_part, then slug. */
export function publishedPosts(posts: Post[] = loadPosts()): Post[] {
  return posts
    .filter((p) => !p.draft)
    .sort(
      (a, b) =>
        b.date.localeCompare(a.date) || (b.part ?? 0) - (a.part ?? 0) || (a.slug < b.slug ? -1 : a.slug > b.slug ? 1 : 0),
    )
}

/** The published posts of the series, in part order. */
export const series = (published: Post[]): Post[] =>
  published.filter((p) => p.part !== undefined).sort((a, b) => a.part! - b.part!)

/** What a post's page shows, as plain data the page's component reads from its route's params. */
export function postView(post: Post, published: Post[]) {
  const parts = series(published)
  const at = post.part === undefined ? -1 : parts.findIndex((p) => p.slug === post.slug)
  const ref = (p?: Post) => (p ? { slug: p.slug, title: p.title } : null)
  return {
    slug: post.slug,
    title: post.title,
    summary: post.summary,
    date: post.date,
    dateLabel: dateLabel(post.date),
    minutes: post.minutes,
    tags: post.tags,
    part: post.part ?? null,
    series: at < 0 ? [] : parts.map((p) => ({ slug: p.slug, title: p.title, part: p.part! })),
    prev: ref(at > 0 ? parts[at - 1] : undefined),
    next: ref(at >= 0 ? parts[at + 1] : undefined),
  }
}

/** A post's text with the links between posts resolved (`blog-post-page`, `agent-markdown`): a
 * link `<slug>.md` to a published post leads to its page, or in a Markdown copy to its copy; a
 * link to a draft is left as its text; one to no post fails the build. In a copy, any other
 * relative link names its file on GitHub's `main`, as a guide copy's does. */
export function resolvePostLinks(
  post: Post,
  all: Post[],
  mode: { kind: 'page' } | { kind: 'copy'; siteUrl: string; repo: string },
): string {
  let fenced = false
  return post.body
    .split('\n')
    .map((line) => {
      if (/^\s*(```|~~~)/.test(line)) fenced = !fenced
      if (fenced) return line
      return line.replace(
        /\[([^\]]*)\]\(([^)\s]+)((?:\s+"[^"]*")?)\)/g,
        (whole, label: string, target: string) => {
          if (/^([a-z][a-z0-9+.-]*:|#|\/)/i.test(target)) return whole
          const hash = target.indexOf('#')
          const path = hash < 0 ? target : target.slice(0, hash)
          const anchor = hash < 0 ? '' : target.slice(hash)
          const sibling = /^([^/?]+)\.md$/.exec(path)
          if (sibling) {
            const other = all.find((p) => p.slug === sibling[1])
            if (!other) throw new Error(`${post.file}: the link to ${target} names no post`)
            if (other.draft) return label
            return mode.kind === 'page'
              ? `[${label}](/blog/${other.slug}${anchor})`
              : `[${label}](${mode.siteUrl}/blog/${other.slug}.md${anchor})`
          }
          if (mode.kind === 'page' || path === '') return whole
          const inRepo = posix.normalize(posix.join('web/site/blog', path))
          return `[${label}](${repoLink(inRepo, path.endsWith('/'), anchor, mode.repo)})`
        },
      )
    })
    .join('\n')
}

/** A post's Markdown copy: its text under a `#` line with its title (`agent-markdown`). */
export const postCopy = (post: Post, all: Post[], siteUrl: string, repo: string): string =>
  `# ${post.title}\n\n${resolvePostLinks(post, all, { kind: 'copy', siteUrl, repo }).replace(/^\s+/, '')}`
