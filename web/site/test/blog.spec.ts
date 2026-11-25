// The site is built from this repository's guide, so an edit to it selects this file
// (`doc-readers-marked`):
// clm:reads docs/guide/**
// clm:reads docs/spec-format.md
import { test, expect, type Page } from '@playwright/test'
import { readFileSync } from 'node:fs'
import { join } from 'node:path'
import { resolvePostLinks } from '../.vitepress/blog'
import type { Post } from '../.vitepress/blog'
import { blogFixture, build, filesUnder, post, read, serve, siteDir, words } from './helpers'

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

// The title and summary of a published post whose text a feed, a head or JSON-LD must escape
// (`blog-feed`'s "markup in a title").
const MARKUP_TITLE = 'A & <B> "C"'
const MARKUP_SUMMARY = 'x < "y" </script>'
// The slug of a published post that no URL can carry as it is (`blog-post-page`'s "an odd slug").
const ODD = 'a&b c'
const ODD_PATH = '/blog/a%26b%20c'

// A blog of five published posts, a draft and a post with no `draft` key; the first two are
// parts 1 and 2 of a series whose part 3 is the draft, and the other three have no part.
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
  'markup.md': post({ ...front(MARKUP_TITLE, '2026-11-03'), summary: MARKUP_SUMMARY }, 'Markup body.'),
  [`${ODD}.md`]: post(front('Odd', '2026-11-02'), 'Odd body.'),
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
  // req:web.site/blog-drafts-left-out@26712d11
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
  // req:web.site/blog-drafts-left-out@26712d11
  // req:web.site/blog-post-page@97174ad7
  const good = front('Good', '2026-11-05')
  const bad: Record<string, string> = {
    'soon.md': post({ ...good, date: 'soon' }, 'x'),
    'impossible.md': post({ ...good, date: '2026-02-30' }, 'x'),
    'month13.md': post({ ...good, date: '2026-13-01' }, 'x'),
    'untitled.md': post({ summary: 's', date: '2026-11-05', draft: false }, 'x'),
    'blanksummary.md': post({ ...good, summary: '  ' }, 'x'),
    'nodate.md': post({ title: 't', summary: 's', draft: false }, 'x'),
    'maybe.md': post({ ...good, draft: 'yes' }, 'x'),
    'tagstring.md': post({ ...good, tags: 'testing' }, 'x'),
    'zero.md': post({ ...good, series_part: 0 }, 'x'),
    'fraction.md': post({ ...good, series_part: 1.5 }, 'x'),
    'nofront.md': '# No frontmatter\n\nJust text.\n',
    'notyaml.md': '---\ntitle: [unclosed\nsummary: s\n---\n\nx\n',
    'list.md': '---\n- title\n- summary\n---\n\nx\n',
    'draftsoon.md': post({ ...good, date: 'soon', draft: true }, 'x'),
    'index.md': post(good, 'x'),
    'feed.md': post(good, 'x'),
    // `blog-post-page`: a link to `<slug>.md` where no post has that slug.
    'nowhere.md': post(good, 'See [x](no-such-post.md).'),
  }
  for (const [file, text] of Object.entries(bad)) {
    const { error } = build('badpost', { BLOG_DIR: blogFixture('badpost', { [file]: text }) })
    expect(error, file).toBeDefined()
    expect(error, file).toContain(file)
  }
  // Two published posts in one place of the series: the failure names both.
  const { error } = build('badpost', {
    BLOG_DIR: blogFixture('badpost', {
      'one.md': post({ ...good, series_part: 1 }, 'x'),
      'two.md': post({ ...good, series_part: 1 }, 'x'),
    }),
  })
  expect(error).toContain('one.md')
  expect(error).toContain('two.md')
  // A draft's duplicate part is no clash.
  const draftPart = build('badpost', {
    BLOG_DIR: blogFixture('badpost', {
      'one.md': post({ ...good, series_part: 1 }, 'x'),
      'two.md': post({ ...good, series_part: 1, draft: true }, 'x'),
    }),
  })
  expect(draftPart.error).toBeUndefined()
})

test('/blog/ shows the published posts as cards, newest first', async ({ page }) => {
  // req:web.site/blog-index@c32d30ca
  await page.goto(`${server.url}/blog/`)
  const cards = page.locator('.blog-card')
  await expect(cards).toHaveCount(5)
  const texts = await cards.allInnerTexts()
  expect(texts.map((t) => t.split('\n').find((l) => /^(Delta|Beta|Alpha|Odd|A & )/.test(l)))).toEqual([
    'Delta',
    'Beta',
    'Alpha',
    MARKUP_TITLE,
    'Odd',
  ])
  for (const want of ['Part 2', '8 Nov 2026', '2 min read', 'Beta summary.', 'testing', 'traceability']) {
    expect(texts[1], want).toContain(want)
  }
  expect(texts[2]).toContain('1 min read')
  expect(texts[2]).toContain('Part 1')
  expect(texts[0]).not.toContain('Part')
  await expect(cards.nth(1)).toHaveAttribute('href', '/blog/beta')
})

test('/blog/ orders posts of one day by part, highest first, then those with none by slug', () => {
  test.setTimeout(240_000)
  // req:web.site/blog-index@c32d30ca
  const day = '2026-11-05'
  const dir = blogFixture('sameday', {
    'a.md': post(front('Post a', day, { series_part: 1 }), 'x'),
    'b.md': post(front('Post b', day, { series_part: 2 }), 'x'),
    'd.md': post(front('Post d', day), 'x'),
    'c.md': post(front('Post c', day), 'x'),
  })
  const { out, error } = build('sameday', { BLOG_DIR: dir })
  expect(error).toBeUndefined()
  const hrefs = Array.from(read(out, 'blog/index.html').matchAll(/class="blog-card" href="([^"]+)"/g), (m) => m[1])
  expect(hrefs).toEqual(['/blog/b', '/blog/a', '/blog/c', '/blog/d'])
})

test('/blog/ with nothing published holds no card and says posts are coming', () => {
  test.setTimeout(240_000)
  // req:web.site/blog-index@c32d30ca
  // req:web.site/structured-data@7aab789a
  // req:web.site/blog-feed@70788316
  // req:web.site/agent-markdown@e6e9273a
  const cases = {
    'only drafts': blogFixture('drafts', {
      'quokka-draft.md': post(front('Quokka draft title', '2026-11-12', { draft: true }), 'zebraphrase'),
    }),
    'an empty directory': blogFixture('empty', {}),
    'no directory': join(siteDir, '.test-dist', 'no-such-blog'),
  }
  for (const [label, dir] of Object.entries(cases)) {
    const { out, error } = build('drafts', { BLOG_DIR: dir })
    expect(error, label).toBeUndefined()
    const html = read(out, 'blog/index.html')
    expect(html, label).not.toContain('blog-card')
    expect(html, label).toContain('Posts are coming')
    expect(ldJson(html)[0].blogPost, label).toEqual([])
    expect(read(out, 'blog/feed.xml'), label).not.toContain('<item>')
    expect(read(out, 'llms.txt'), label).not.toContain('## Blog')
  }
})

test('the cards stand in one column on a phone and nothing scrolls sideways', async ({ page }) => {
  // req:web.site/blog-index@c32d30ca
  await page.setViewportSize({ width: 390, height: 800 })
  await page.goto(`${server.url}/blog/`)
  const boxes = await page.locator('.blog-card').evaluateAll((els) =>
    els.map((e) => {
      const r = e.getBoundingClientRect()
      return { left: r.left, top: r.top, right: r.right }
    }),
  )
  expect(boxes.length).toBe(5)
  for (const b of boxes) expect(b.left).toBe(boxes[0].left)
  expect(boxes[1].top).toBeGreaterThan(boxes[0].top)
  const widths = await page.evaluate(() => ({ doc: document.documentElement.scrollWidth, win: window.innerWidth }))
  expect(widths.doc).toBeLessThanOrEqual(widths.win)
  expect(Math.max(...boxes.map((b) => b.right))).toBeLessThanOrEqual(widths.win)
})

test.describe('with script disabled', () => {
  test.use({ javaScriptEnabled: false })

  test("a post's page holds its text, its series without the draft, and its neighbours", async ({ page }) => {
    // req:web.site/blog-post-page@97174ad7
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
    // req:web.site/blog-post-page@97174ad7
    await page.goto(`${server.url}/blog/alpha`)
    await expect(page.locator('.post-body a', { hasText: 'Beta' })).toHaveAttribute('href', '/blog/beta')
    await expect(page.locator('.post-nav .next')).toHaveAttribute('href', '/blog/beta')
    await expect(page.locator('.post-nav .prev')).toHaveCount(0)
  })

  test('a post with no part shows no series and no neighbours', async ({ page }) => {
    // req:web.site/blog-post-page@97174ad7
    await page.goto(`${server.url}/blog/delta`)
    await expect(page.locator('.clm-post h1')).toHaveText('Delta')
    await expect(page.locator('.post-series')).toHaveCount(0)
    await expect(page.locator('.post-nav')).toHaveCount(0)
  })

  test("a slug a URL can't carry is percent-encoded wherever the site writes it", async ({ page }) => {
    // req:web.site/blog-post-page@97174ad7
    const url = SITE + ODD_PATH
    const feed = await parseFeed(page, read(seriesOut, 'blog/feed.xml'))
    expect(feed).not.toBeNull()
    expect(feed!.find((i) => i.title === 'Odd')?.link).toBe(url)
    expect(sitemapUrls(seriesOut)).toContain(url)
    const html = read(seriesOut, `blog/${ODD}.html`)
    expect(html).toContain(`<link rel="canonical" href="${url}">`)
    expect(html).toContain(`<link rel="alternate" type="text/markdown" href="${url}.md">`)
    expect(ldJson(html).find((d) => d['@type'] === 'BlogPosting').url).toBe(url)
    const [blog] = ldJson(read(seriesOut, 'blog/index.html'))
    expect(blog.blogPost.map((p: { url: string }) => p.url)).toContain(url)
    expect(read(seriesOut, 'llms.txt')).toContain(`- [Odd](${url}.md): Odd summary.`)
    await page.goto(`${server.url}/blog/`)
    const card = page.locator('.blog-card', { hasText: 'Odd' })
    await expect(card).toHaveAttribute('href', ODD_PATH)
    await card.click()
    await expect(page.locator('.clm-post h1')).toHaveText('Odd')
  })
})

/** The feed's items, as the browser's XML parser reads them, or null when it isn't well-formed. */
function parseFeed(page: Page, xml: string) {
  return page.evaluate((text) => {
    const doc = new DOMParser().parseFromString(text, 'application/xml')
    if (doc.querySelector('parsererror')) return null
    const field = (item: Element, name: string) => item.querySelector(name)?.textContent ?? null
    return Array.from(doc.querySelectorAll('item'), (item) => ({
      title: field(item, 'title'),
      link: field(item, 'link'),
      guid: field(item, 'guid'),
      description: field(item, 'description'),
    }))
  }, xml)
}

const sitemapUrls = (out: string) =>
  Array.from(read(out, 'sitemap.xml').matchAll(/<loc>([^<]+)<\/loc>/g), (m) => m[1])

test('a title and summary holding markup are read as written in the feed, the heads and JSON-LD', async ({
  page,
}) => {
  // req:web.site/blog-feed@70788316
  // req:web.site/page-metadata@8d1c2456
  const feed = await parseFeed(page, read(seriesOut, 'blog/feed.xml'))
  expect(feed).not.toBeNull()
  const item = feed!.find((i) => i.title === MARKUP_TITLE)
  expect(item?.description).toBe(MARKUP_SUMMARY)
  const html = read(seriesOut, 'blog/markup.html')
  expect(ldJson(html).find((d) => d['@type'] === 'BlogPosting').headline).toBe(MARKUP_TITLE)
  const [blog] = ldJson(read(seriesOut, 'blog/index.html'))
  expect(blog.blogPost.map((p: { headline: string }) => p.headline)).toContain(MARKUP_TITLE)
  await page.goto(`${server.url}/blog/markup`)
  const head = await page.evaluate(() => ({
    descriptions: Array.from(document.querySelectorAll('meta[name="description"]'), (m) =>
      m.getAttribute('content'),
    ),
    og: document.querySelector('meta[property="og:description"]')?.getAttribute('content'),
  }))
  expect(head).toEqual({ descriptions: [MARKUP_SUMMARY], og: MARKUP_SUMMARY })
})

test("the feed has an item for each published post, newest first, and the heads link it", async ({ page }) => {
  // req:web.site/blog-feed@70788316
  const feed = read(seriesOut, 'blog/feed.xml')
  expect(feed).toContain('<rss version="2.0">')
  const parsed = await parseFeed(page, feed)
  expect(parsed!.map((i) => i.title)).toEqual(['Delta', 'Beta', 'Alpha', MARKUP_TITLE, 'Odd'])
  expect(parsed![1]).toMatchObject({ link: `${SITE}/blog/beta`, guid: `${SITE}/blog/beta`, description: 'Beta summary.' })
  const items = feed.split('<item>').slice(1)
  expect(items[1]).toContain('<pubDate>Sun, 08 Nov 2026 00:00:00 GMT</pubDate>')
  const link = `<link rel="alternate" type="application/rss+xml" title="CLM blog" href="${SITE}/blog/feed.xml">`
  expect(read(seriesOut, 'blog/index.html')).toContain(link)
  expect(read(seriesOut, 'blog/alpha.html')).toContain(link)
})

test("a post's head holds its title, summary, canonical link, article type and Markdown copy", () => {
  // req:web.site/page-metadata@8d1c2456
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
  // req:web.site/blog-updated-date@52adaca2
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
    'month13.md': post({ ...good, updated: '2026-13-01' }, 'x'),
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
  // req:web.site/agent-markdown@e6e9273a
  const draft = { slug: 'next', draft: true } as Post
  const body = 'See [the readme](../../../README.md#top), [next](next.md) and ![pic](next.md).'
  const alpha = { slug: 'alpha', file: 'alpha.md', draft: false, body } as Post
  const repo = 'someone/clm-public'
  const copy = resolvePostLinks(alpha, [alpha, draft], repo, { kind: 'copy', siteUrl: SITE })
  expect(copy).toBe(
    `See [the readme](https://github.com/${repo}/blob/main/README.md#top), next and ![pic](next.md).`,
  )
})

test('the sitemap lists /blog/ and the published posts, and no draft', () => {
  // req:web.site/crawler-files@08777c25
  const urls = sitemapUrls(seriesOut)
  for (const path of ['/blog/', '/blog/alpha', '/blog/beta', '/blog/delta']) expect(urls).toContain(SITE + path)
  expect(urls.filter((u) => /quokka|nokey|feed/.test(u))).toEqual([])
})

test('structured data describes the blog and each post', () => {
  // req:web.site/structured-data@7aab789a
  const [blog] = ldJson(read(seriesOut, 'blog/index.html'))
  expect(blog['@type']).toBe('Blog')
  expect(blog.blogPost.map((p: { url: string }) => p.url)).toEqual(
    ['delta', 'beta', 'alpha', 'markup', 'a%26b%20c'].map((s) => `${SITE}/blog/${s}`),
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
  // req:web.site/structured-data@7aab789a
  // req:web.site/crawler-files@08777c25
  // req:web.site/agent-markdown@e6e9273a
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
  // req:web.site/agent-markdown@e6e9273a
  const llms = read(seriesOut, 'llms.txt')
  const blog = llms.slice(llms.indexOf('## Blog'), llms.indexOf('## Optional'))
  expect(blog.split('\n').filter((l) => l.startsWith('- '))).toEqual([
    `- [Delta](${SITE}/blog/delta.md): Delta summary.`,
    `- [Beta](${SITE}/blog/beta.md): Beta summary.`,
    `- [Alpha](${SITE}/blog/alpha.md): First post.`,
    `- [${MARKUP_TITLE}](${SITE}/blog/markup.md): ${MARKUP_SUMMARY}`,
    `- [Odd](${SITE}${ODD_PATH}.md): Odd summary.`,
  ])
  expect(llms.indexOf('## Guide')).toBeLessThan(llms.indexOf('## Blog'))
  // `llms-full.txt` holds the guide only.
  expect(read(seriesOut, 'llms-full.txt')).not.toContain('Alpha')
  const copy = read(seriesOut, 'blog/alpha.md')
  expect(copy.startsWith('# Alpha\n\n')).toBe(true)
  expect(copy).toContain(`[Beta](${SITE}/blog/beta.md)`)
  expect(copy).toContain('and draft.')
})
