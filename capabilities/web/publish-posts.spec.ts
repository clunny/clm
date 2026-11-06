// `publish-posts`: the maintainer keeps posts as drafts and publishes them one at a time. The site
// is built from a fixture blog of its own (`BLOG_DIR`), as the site workflow builds it, and served
// as static files; then a draft is published and the site is built again. No Worker is involved.
import { execFileSync } from 'node:child_process'
import { existsSync, mkdirSync, mkdtempSync, readFileSync, readdirSync, rmSync, statSync, writeFileSync } from 'node:fs'
import { createServer, type Server } from 'node:http'
import { tmpdir } from 'node:os'
import { dirname, extname, join } from 'node:path'
import { fileURLToPath } from 'node:url'
import { expect, test } from '@playwright/test'

const root = join(dirname(fileURLToPath(import.meta.url)), '..', '..')
const SITE = 'https://clm-app.dev'
const DRAFT_TITLE = 'Quokka finale'
const DRAFT_BODY = 'The finale says zebraphrase.'

const TYPES: Record<string, string> = {
  '.html': 'text/html',
  '.js': 'text/javascript',
  '.css': 'text/css',
  '.json': 'application/json',
  '.xml': 'application/xml',
  '.txt': 'text/plain',
  '.md': 'text/markdown',
}

/** A post's file: the frontmatter, then the body. */
function post(title: string, part: number, date: string, draft: boolean, body: string): string {
  const front = { title, summary: `${title} summary.`, date, tags: ['series'], series_part: part, draft }
  const lines = Object.entries(front).map(([k, v]) => `${k}: ${JSON.stringify(v)}`)
  return `---\n${lines.join('\n')}\n---\n\n${body}\n`
}

/** Builds the site as the site workflow does, with the blog read from `blog` and written to `out`. */
function build(blog: string, out: string) {
  const env = { ...process.env, BLOG_DIR: blog, SITE_OUT: out, GITHUB_STARS: '1234', GITHUB_API: 'http://127.0.0.1:9' }
  for (const key of ['UMAMI_WEBSITE_ID', 'INSTALL_REPO', 'GUIDE_DIR']) delete env[key]
  execFileSync('npx', ['vitepress', 'build', '.'], { cwd: join(root, 'web', 'site'), env, stdio: 'pipe', maxBuffer: 1 << 28 })
}

/** Serves `dir` as static files, with `/x` answered by `x.html` as the deployment does. */
function serve(dir: string): Promise<{ base: string; server: Server }> {
  const server = createServer((req, res) => {
    const path = decodeURIComponent(new URL(req.url!, 'http://x').pathname)
    const candidates = [path, `${path}.html`, join(path, 'index.html')]
    const file = candidates.map((c) => join(dir, c)).find((f) => existsSync(f) && statSync(f).isFile())
    if (!file) {
      res.writeHead(404).end('not found')
      return
    }
    res.writeHead(200, { 'content-type': TYPES[extname(file)] ?? 'application/octet-stream' }).end(readFileSync(file))
  })
  return new Promise((resolve) => {
    server.listen(0, '127.0.0.1', () => {
      const address = server.address() as { port: number }
      resolve({ base: `http://127.0.0.1:${address.port}`, server })
    })
  })
}

const filesUnder = (dir: string, rel = ''): string[] =>
  readdirSync(join(dir, rel)).flatMap((name) => {
    const path = join(rel, name)
    return statSync(join(dir, path)).isDirectory() ? filesUnder(dir, path) : [path]
  })

/** What a crawler reads: the sitemap's locations, `llms.txt` and the feed. */
async function crawled(base: string) {
  const get = async (path: string) => (await fetch(`${base}${path}`)).text()
  return {
    sitemap: Array.from((await get('/sitemap.xml')).matchAll(/<loc>([^<]+)<\/loc>/g), (m) => m[1]),
    llms: await get('/llms.txt'),
    feed: await get('/blog/feed.xml'),
  }
}

test('a series is published one post at a time, and a draft shows nowhere', async ({ page }) => {
  // req:capability/publish-posts@20404270
  test.setTimeout(300_000)
  const work = mkdtempSync(join(tmpdir(), 'clm-publish-posts-'))
  const blog = join(work, 'blog')
  mkdirSync(blog)
  writeFileSync(join(blog, 'part-one.md'), post('Part one', 1, '2026-11-05', false, 'The start. Next is [part two](part-two.md).'))
  writeFileSync(join(blog, 'part-two.md'), post('Part two', 2, '2026-11-08', false, `Middle. Then [the finale](quokka-finale.md).`))
  writeFileSync(join(blog, 'quokka-finale.md'), post(DRAFT_TITLE, 3, '2026-11-12', true, DRAFT_BODY))
  let server: Server | undefined
  try {
    // As the site workflow builds it, with the draft still a draft.
    const before = join(work, 'before')
    build(blog, before)
    const served = await serve(before)
    server = served.server

    await page.goto(`${served.base}/blog/`)
    const cards = page.locator('.blog-card')
    await expect(cards).toHaveCount(2)
    expect((await cards.allInnerTexts()).join('\n')).toContain('Part two')
    await expect(page.locator('.blog-card[href="/blog/part-two"]')).toHaveCount(1)
    await expect(page.locator('.blog-card[href="/blog/part-one"]')).toHaveCount(1)

    await page.goto(`${served.base}/blog/part-two`)
    const main = page.locator('.clm-post')
    await expect(main.locator('h1')).toHaveText('Part two')
    await expect(main.locator('.post-nav .prev')).toHaveAttribute('href', '/blog/part-one')
    await expect(main.locator('.post-nav .next')).toHaveCount(0)

    const first = await crawled(served.base)
    for (const slug of ['part-one', 'part-two']) {
      expect(first.sitemap).toContain(`${SITE}/blog/${slug}`)
      expect(first.llms).toContain(`${SITE}/blog/${slug}.md`)
      expect(first.feed).toContain(`<link>${SITE}/blog/${slug}</link>`)
    }
    expect(first.sitemap).toContain(`${SITE}/blog/`)
    // The draft's title and text appear in no file the build wrote.
    for (const file of filesUnder(before)) {
      expect(file, file).not.toMatch(/quokka/i)
      const text = readFileSync(join(before, file), 'latin1')
      for (const secret of [DRAFT_TITLE, 'zebraphrase', 'quokka-finale']) expect(text.includes(secret), `${file} holds ${secret}`).toBe(false)
    }

    // Publish the third part and build again.
    writeFileSync(join(blog, 'quokka-finale.md'), post(DRAFT_TITLE, 3, '2026-11-12', false, DRAFT_BODY))
    const after = join(work, 'after')
    build(blog, after)
    const servedAfter = await serve(after)
    const firstServer = server
    server = servedAfter.server
    firstServer.close()

    await page.goto(`${servedAfter.base}/blog/`)
    await expect(page.locator('.blog-card')).toHaveCount(3)
    await expect(page.locator('.blog-card[href="/blog/quokka-finale"]')).toHaveCount(1)
    await page.goto(`${servedAfter.base}/blog/quokka-finale`)
    await expect(page.locator('.clm-post h1')).toHaveText(DRAFT_TITLE)

    await page.goto(`${servedAfter.base}/blog/part-two`)
    await expect(page.locator('.clm-post .post-nav .next')).toHaveAttribute('href', '/blog/quokka-finale')
    await expect(page.locator('.clm-post .post-body a', { hasText: 'the finale' })).toHaveAttribute('href', '/blog/quokka-finale')

    const second = await crawled(servedAfter.base)
    expect(second.sitemap).toContain(`${SITE}/blog/quokka-finale`)
    expect(second.llms).toContain(`[${DRAFT_TITLE}](${SITE}/blog/quokka-finale.md)`)
    expect(second.feed).toContain(`<link>${SITE}/blog/quokka-finale</link>`)
    expect(readFileSync(join(after, 'blog', 'quokka-finale.md'), 'utf8')).toContain(DRAFT_BODY)
  } finally {
    server?.close()
    rmSync(work, { recursive: true, force: true })
  }
})
