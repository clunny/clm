// The site is built from this repository's guide, so an edit to it selects this file
// (`doc-readers-marked`):
// clm:reads docs/guide/**
// clm:reads docs/spec-format.md
import { test, expect } from '@playwright/test'
import { readFileSync } from 'node:fs'
import { join } from 'node:path'
import { resolvePostLinks } from '../.vitepress/blog'
import type { Post } from '../.vitepress/blog'
import { blogFixture, build, filesUnder, post, read, serve, words } from './helpers'

const SITE = 'https://clm-app.dev'
const meta = (html: string, attr: 'name' | 'property', key: string) =>
  new RegExp(`<meta ${attr}="${key}" content="([^"]*)"`).exec(html)?.[1]
const title = (html: string) => /<title>([^<]*)<\/title>/.exec(html)?.[1]
const ldJson = (html: string) =>
  Array.from(html.matchAll(/<script type="application\/ld\+json">([^<]*)<\/script>/g), (m) => JSON.parse(m[1]))

const front = (title: string, date: string, extra: Record<string, string | number | boolean | string[]> = {}) => ({
  title,
  summary: `${title} summary.`,
  date,
  tags: ['testing'],
  draft: false,
  ...extra,
})

// A blog of three published posts, a draft and a post with no `draft` key; the first two are
// parts 1 and 2 of a series whose part 3 is the draft.
const series = {
  'alpha.md': post(
    { ...front('Alpha', '2026-11-05', { series_part: 1, updated: '2026-11-09' }), summary: 'First post.' },
    `See [Beta](beta.md) and [draft](quokka-draft.md). ${words(226)}`,
  ),
  'beta.md': post(
    front('Beta', '2026-11-08', { series_part: 2, tags: ['testing', 'traceability'] }),
    `${words(229)}\n\n[Part three](quokka-draft.md)`,
  ),
  'delta.md': post(front('Delta', '2026-11-10', { tags: ['misc'] }), 'Delta body.'),
  'quokka-draft.md': post(
    front('Quokka draft title', '2026-11-12', { series_part: 3, draft: true }),
    'The draft says zebraphrase.',
  ),
  'nokey.md': post({ title: 'Nokey title', summary: 'Nokey summary.', date: '2026-11-13', tags: [] }, 'No draft key.'),
}

test.describe.configure({ mode: 'serial' })

let seriesOut = ''
let server: Awaited<ReturnType<typeof serve>>

test.beforeAll(async () => {
  test.setTimeout(240_000)
  const { out, error } = build('series', { BLOG_DIR: blogFixture('series', series), GITHUB_STARS: '1234' })
  expect(error).toBeUndefined()
  seriesOut = out
  server = await serve(out)
})

test.afterAll(async () => {
  await server?.close()
})

test('a draft leaves no trace in any built file', () => {
  // req:web.site/blog-drafts-left-out@b80f5701
  const files = filesUnder(seriesOut)
  expect(files).toContain('blog/alpha.html')
  expect(files.filter((f) => /quokka|nokey/i.test(f))).toEqual([])
  for (const f of files) {
    const text = readFileSync(join(seriesOut, f), 'latin1')
    for (const secret of ['quokka', 'Quokka draft title', 'zebraphrase', 'Nokey title', 'No draft key']) {
      expect(text.includes(secret), `${f} holds ${secret}`).toBe(false)
    }
  }
})

test('a post that is missing a field or has a bad one fails the build, naming its file', () => {
  test.setTimeout(240_000)
  // req:web.site/blog-drafts-left-out@b80f5701
  const good = front('Good', '2026-11-05')
  const bad: Record<string, string> = {
    'soon.md': post({ ...good, date: 'soon' }, 'x'),
    'impossible.md': post({ ...good, date: '2026-02-30' }, 'x'),
    'untitled.md': post({ summary: 's', date: '2026-11-05', draft: false }, 'x'),
    'nosummary.md': post({ title: 't', date: '2026-11-05', draft: false }, 'x'),
    'nodate.md': post({ title: 't', summary: 's', draft: false }, 'x'),
    'maybe.md': post({ ...good, draft: 'yes' }, 'x'),
    'tagstring.md': post({ ...good, tags: 'testing' }, 'x'),
    'zero.md': post({ ...good, series_part: 0 }, 'x'),
    'index.md': post(good, 'x'),
    'feed.md': post(good, 'x'),
    'nowhere.md': post(good, 'See [x](no-such-post.md).'),
  }
  for (const [file, text] of Object.entries(bad)) {
    const { error } = build('badpost', { BLOG_DIR: blogFixture('badpost', { [file]: text }) })
    expect(error, file).toBeDefined()
    expect(error, file).toContain(file)
  }
  // Two published posts in one place of the series.
  const { error } = build('badpost', {
    BLOG_DIR: blogFixture('badpost', {
      'one.md': post({ ...good, series_part: 1 }, 'x'),
      'two.md': post({ ...good, series_part: 1 }, 'x'),
    }),
  })
  expect(error).toContain('series_part 1')
  // A draft is checked too, and a draft's duplicate part is no clash.
  const draftClash = build('badpost', {
    BLOG_DIR: blogFixture('badpost', {
      'one.md': post({ ...good, series_part: 1 }, 'x'),
      'two.md': post({ ...good, series_part: 1, draft: true }, 'x'),
      'three.md': post({ ...good, date: 'soon', draft: true }, 'x'),
    }),
  })
  expect(draftClash.error).toContain('three.md')
})

test('/blog/ shows the published posts as cards, newest first', async ({ page }) => {
  // req:web.site/blog-index@4dfcf759
  await page.goto(`${server.url}/blog/`)
  const cards = page.locator('.blog-card')
  await expect(cards).toHaveCount(3)
  const texts = await cards.allInnerTexts()
  expect(texts[0]).toContain('Delta')
  expect(texts[1]).toContain('Beta')
  expect(texts[2]).toContain('Alpha')
  for (const want of ['Part 2', '8 Nov 2026', '2 min read', 'Beta summary.', 'testing', 'traceability']) {
    expect(texts[1], want).toContain(want)
  }
  expect(texts[2]).toContain('1 min read')
  expect(texts[2]).toContain('Part 1')
  expect(texts[0]).not.toContain('Part')
  await expect(cards.nth(1)).toHaveAttribute('href', '/blog/beta')
  // The nav of every page links to the blog after Guide.
  const nav = (await page.locator('.VPNavBarMenu a').allTextContents()).map((t) => t.trim())
  expect(nav.indexOf('Blog')).toBe(nav.indexOf('Guide') + 1)
})

test('/blog/ with nothing published holds no card and says posts are coming', () => {
  test.setTimeout(240_000)
  // req:web.site/blog-index@4dfcf759
  // req:web.site/structured-data@56e1b2db
  // req:web.site/blog-feed@cdf76160
  // req:web.site/agent-markdown@6ed9b7f5
  const dir = blogFixture('drafts', {
    'quokka-draft.md': post(front('Quokka draft title', '2026-11-12', { draft: true }), 'zebraphrase'),
  })
  const { out, error } = build('drafts', { BLOG_DIR: dir })
  expect(error).toBeUndefined()
  const html = read(out, 'blog/index.html')
  expect(html).not.toContain('blog-card')
  expect(html).toContain('Posts are coming')
  expect(ldJson(html)[0].blogPost).toEqual([])
  expect(read(out, 'blog/feed.xml')).not.toContain('<item>')
  expect(read(out, 'llms.txt')).not.toContain('## Blog')
})

test('the cards stand in one column on a phone and nothing scrolls sideways', async ({ page }) => {
  // req:web.site/blog-index@4dfcf759
  await page.setViewportSize({ width: 390, height: 800 })
  await page.goto(`${server.url}/blog/`)
  const boxes = await page.locator('.blog-card').evaluateAll((els) =>
    els.map((e) => {
      const r = e.getBoundingClientRect()
      return { left: r.left, top: r.top, right: r.right }
    }),
  )
  expect(boxes.length).toBe(3)
  for (const b of boxes) expect(b.left).toBe(boxes[0].left)
  expect(boxes[1].top).toBeGreaterThan(boxes[0].top)
  const widths = await page.evaluate(() => ({ doc: document.documentElement.scrollWidth, win: window.innerWidth }))
  expect(widths.doc).toBeLessThanOrEqual(widths.win)
  expect(Math.max(...boxes.map((b) => b.right))).toBeLessThanOrEqual(widths.win)
})

test.describe('with script disabled', () => {
  test.use({ javaScriptEnabled: false })

  test("a post's page holds its text, its series without the draft, and its neighbours", async ({ page }) => {
    // req:web.site/blog-post-page@137f3ed3
    await page.goto(`${server.url}/blog/beta`)
    const main = page.locator('.clm-post')
    await expect(main.locator('h1')).toHaveText('Beta')
    await expect(main.locator('.lede')).toHaveText('Beta summary.')
    const byline = await main.locator('.byline').innerText()
    for (const want of ['8 Nov 2026', '2 min read', 'testing, traceability']) expect(byline).toContain(want)
    await expect(main.locator('.post-body')).toContainText('word word')
    const items = main.locator('.post-series li')
    expect(await items.allInnerTexts()).toEqual(['Alpha', 'Beta'])
    await expect(main.locator('.post-series li.cur')).toHaveText('Beta')
    await expect(main.locator('.post-nav .prev')).toHaveAttribute('href', '/blog/alpha')
    await expect(main.locator('.post-nav .next')).toHaveCount(0)
    // The link to the draft is its text alone.
    await expect(main.locator('.post-body')).toContainText('Part three')
    await expect(main.locator('.post-body a', { hasText: 'Part three' })).toHaveCount(0)
    expect(await page.content()).not.toContain('quokka')
  })

  test('a link to a published post leads to its page', async ({ page }) => {
    // req:web.site/blog-post-page@137f3ed3
    await page.goto(`${server.url}/blog/alpha`)
    await expect(page.locator('.post-body a', { hasText: 'Beta' })).toHaveAttribute('href', '/blog/beta')
    await expect(page.locator('.post-nav .next')).toHaveAttribute('href', '/blog/beta')
    await expect(page.locator('.post-nav .prev')).toHaveCount(0)
  })
})

test("the feed has an item for each published post, newest first, and the heads link it", () => {
  // req:web.site/blog-feed@cdf76160
  const feed = read(seriesOut, 'blog/feed.xml')
  expect(feed).toContain('<rss version="2.0">')
  const items = feed.split('<item>').slice(1)
  expect(items.length).toBe(3)
  expect(items.map((i) => /<title>([^<]*)/.exec(i)![1])).toEqual(['Delta', 'Beta', 'Alpha'])
  expect(items[1]).toContain(`<link>${SITE}/blog/beta</link>`)
  expect(items[1]).toContain(`<guid>${SITE}/blog/beta</guid>`)
  expect(items[1]).toContain('<description>Beta summary.</description>')
  expect(items[1]).toContain('<pubDate>Sun, 08 Nov 2026 00:00:00 GMT</pubDate>')
  const link = `<link rel="alternate" type="application/rss+xml" title="CLM blog" href="${SITE}/blog/feed.xml">`
  expect(read(seriesOut, 'blog/index.html')).toContain(link)
  expect(read(seriesOut, 'blog/alpha.html')).toContain(link)
})

test("a post's head holds its title, summary, canonical link, article type and Markdown copy", () => {
  // req:web.site/page-metadata@3f985405
  const html = read(seriesOut, 'blog/alpha.html')
  expect(title(html)).toBe('Alpha | CLM blog')
  expect(meta(html, 'name', 'description')).toBe('First post.')
  expect(meta(html, 'property', 'og:description')).toBe('First post.')
  expect(html).toContain(`<link rel="canonical" href="${SITE}/blog/alpha">`)
  expect(meta(html, 'property', 'og:type')).toBe('article')
  expect(meta(html, 'property', 'article:published_time')).toBe('2026-11-05')
  expect(html).toContain(`<link rel="alternate" type="text/markdown" href="${SITE}/blog/alpha.md">`)
  const index = read(seriesOut, 'blog/index.html')
  expect(title(index)).toBe('CLM blog')
  expect(meta(index, 'name', 'description')).toContain("What we've learned building CLM with CLM")
  expect(meta(index, 'name', 'description')).not.toContain('Declare the intent')
  expect(index).toContain(`<link rel="canonical" href="${SITE}/blog/">`)
})

test('an updated date shows on the page and in the head, and a bad one fails the build', async ({ page }) => {
  test.setTimeout(240_000)
  // req:web.site/blog-updated-date@11514ec9
  await page.goto(`${server.url}/blog/alpha`)
  const byline = page.locator('.byline')
  await expect(byline).toContainText('5 Nov 2026')
  await expect(byline).toContainText('Updated 9 Nov 2026')
  await expect(byline.locator('time[datetime="2026-11-09"]')).toHaveCount(1)
  const alpha = read(seriesOut, 'blog/alpha.html')
  expect(meta(alpha, 'property', 'article:modified_time')).toBe('2026-11-09')
  const alphaLd = ldJson(alpha).find((d) => d['@type'] === 'BlogPosting')
  expect(alphaLd.datePublished).toBe('2026-11-05')
  expect(alphaLd.dateModified).toBe('2026-11-09')
  // A post with no `updated` shows and holds none of it.
  await page.goto(`${server.url}/blog/beta`)
  await expect(page.locator('.byline')).not.toContainText('Updated')
  const beta = read(seriesOut, 'blog/beta.html')
  expect(meta(beta, 'property', 'article:modified_time')).toBeUndefined()
  expect(ldJson(beta).find((d) => d['@type'] === 'BlogPosting')).not.toHaveProperty('dateModified')
  const good = front('Good', '2026-11-05')
  const bad: Record<string, string> = {
    'early.md': post({ ...good, updated: '2026-11-01' }, 'x'),
    'later.md': post({ ...good, updated: 'later' }, 'x'),
    'impossible.md': post({ ...good, updated: '2026-02-30' }, 'x'),
  }
  for (const [file, text] of Object.entries(bad)) {
    const { error } = build('badupdated', { BLOG_DIR: blogFixture('badupdated', { [file]: text }) })
    expect(error, file).toBeDefined()
    expect(error, file).toContain(file)
  }
  // The same day as its date is allowed.
  const same = build('badupdated', { BLOG_DIR: blogFixture('badupdated', { 'same.md': post({ ...good, updated: '2026-11-05' }, 'x') }) })
  expect(same.error).toBeUndefined()
})

test('in a post copy, a relative link into the repository names GitHub, and an image is no post link', () => {
  // req:web.site/agent-markdown@6ed9b7f5
  const draft = { slug: 'next', draft: true } as Post
  const body = 'See [the readme](../../../README.md#top), [next](next.md) and ![pic](next.md).'
  const alpha = { slug: 'alpha', file: 'alpha.md', draft: false, body } as Post
  const repo = 'someone/clm-public'
  const copy = resolvePostLinks(alpha, [alpha, draft], { kind: 'copy', siteUrl: SITE, repo })
  expect(copy).toBe(
    `See [the readme](https://github.com/${repo}/blob/main/README.md#top), next and ![pic](next.md).`,
  )
})

test('the sitemap lists /blog/ and the published posts, and no draft', () => {
  // req:web.site/crawler-files@7811e987
  const urls = Array.from(read(seriesOut, 'sitemap.xml').matchAll(/<loc>([^<]+)<\/loc>/g), (m) => m[1])
  for (const path of ['/blog/', '/blog/alpha', '/blog/beta', '/blog/delta']) expect(urls).toContain(SITE + path)
  expect(urls.filter((u) => /quokka|nokey|feed/.test(u))).toEqual([])
})

test('structured data describes the blog and each post', () => {
  // req:web.site/structured-data@56e1b2db
  const [blog] = ldJson(read(seriesOut, 'blog/index.html'))
  expect(blog['@type']).toBe('Blog')
  expect(blog.blogPost.map((p: { url: string }) => p.url)).toEqual(
    ['delta', 'beta', 'alpha'].map((s) => `${SITE}/blog/${s}`),
  )
  const [alpha] = ldJson(read(seriesOut, 'blog/alpha.html'))
  expect(alpha['@type']).toBe('BlogPosting')
  expect(alpha.headline).toBe('Alpha')
  expect(alpha.description).toBe('First post.')
  expect(alpha.datePublished).toBe('2026-11-05')
  expect(alpha.keywords).toContain('testing')
  expect(alpha.url).toBe(`${SITE}/blog/alpha`)
  expect(alpha.author).toEqual({ '@type': 'Organization', name: 'CLM' })
})

test('a lone published post is all the blog lists, everywhere', () => {
  test.setTimeout(240_000)
  // req:web.site/structured-data@56e1b2db
  // req:web.site/crawler-files@7811e987
  // req:web.site/agent-markdown@6ed9b7f5
  const dir = blogFixture('one', {
    'alpha.md': post(front('Alpha', '2026-11-05'), 'See [the draft](quokka-draft.md) and [Alpha](alpha.md#top).'),
    'quokka-draft.md': post(front('Quokka draft title', '2026-11-12', { draft: true }), 'zebraphrase'),
  })
  const { out, error } = build('one', { BLOG_DIR: dir })
  expect(error).toBeUndefined()
  const [blog] = ldJson(read(out, 'blog/index.html'))
  expect(blog.blogPost.map((p: { url: string }) => p.url)).toEqual([`${SITE}/blog/alpha`])
  const urls = Array.from(read(out, 'sitemap.xml').matchAll(/<loc>([^<]+)<\/loc>/g), (m) => m[1])
  expect(urls).toContain(`${SITE}/blog/`)
  expect(urls).toContain(`${SITE}/blog/alpha`)
  expect(urls.filter((u) => u.includes('quokka'))).toEqual([])
  const llms = read(out, 'llms.txt')
  expect(llms).toContain(`## Blog\n\n- [Alpha](${SITE}/blog/alpha.md): Alpha summary.`)
  expect(read(out, 'blog/alpha.md')).toBe(
    `# Alpha\n\nSee the draft and [Alpha](${SITE}/blog/alpha.md#top).\n`,
  )
  for (const f of filesUnder(out)) {
    expect(f, f).not.toContain('quokka')
    expect(readFileSync(join(out, f), 'latin1').includes('quokka'), f).toBe(false)
  }
})

test('llms.txt lists the published posts newest first under Blog, before Optional', () => {
  // req:web.site/agent-markdown@6ed9b7f5
  const llms = read(seriesOut, 'llms.txt')
  const blog = llms.slice(llms.indexOf('## Blog'), llms.indexOf('## Optional'))
  expect(blog.split('\n').filter((l) => l.startsWith('- '))).toEqual([
    `- [Delta](${SITE}/blog/delta.md): Delta summary.`,
    `- [Beta](${SITE}/blog/beta.md): Beta summary.`,
    `- [Alpha](${SITE}/blog/alpha.md): First post.`,
  ])
  expect(llms.indexOf('## Guide')).toBeLessThan(llms.indexOf('## Blog'))
  // `llms-full.txt` holds the guide only.
  expect(read(seriesOut, 'llms-full.txt')).not.toContain('Alpha')
  const copy = read(seriesOut, 'blog/alpha.md')
  expect(copy.startsWith('# Alpha\n\n')).toBe(true)
  expect(copy).toContain(`[Beta](${SITE}/blog/beta.md)`)
  expect(copy).toContain('and draft.')
})
