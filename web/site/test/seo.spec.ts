import { test, expect } from '@playwright/test'
import { execFileSync } from 'node:child_process'
import { existsSync, readFileSync, readdirSync, writeFileSync } from 'node:fs'
import { join, resolve } from 'node:path'
import { guideSidebar, rewriteLinks } from '../.vitepress/guide'
import { build, filesUnder, fixture, mainDist, read, repoDocs, siteDir } from './helpers'

const SITE = 'https://clm-app.dev'
const meta = (html: string, attr: 'name' | 'property', key: string) =>
  new RegExp(`<meta ${attr}="${key}" content="([^"]*)"`).exec(html)?.[1]
const title = (html: string) => /<title>([^<]*)<\/title>/.exec(html)?.[1]
const canonical = (html: string) => /<link rel="canonical" href="([^"]*)"/.exec(html)?.[1]
const ldJson = (html: string) =>
  Array.from(html.matchAll(/<script type="application\/ld\+json">([^<]*)<\/script>/g), (m) => JSON.parse(m[1]))

test("a guide page's head holds its metadata and its social card", () => {
  // req:web.site/page-metadata@6cc8ba05
  const html = read(mainDist, 'guide/quickstart.html')
  const sentence = 'From nothing to your first change built, for a new project or an existing one.'
  expect(title(html)).toBe('Quickstart | CLM guide')
  expect(meta(html, 'name', 'description')).toBe(sentence)
  expect(meta(html, 'property', 'og:description')).toBe(sentence)
  expect(meta(html, 'name', 'twitter:description')).toBe(sentence)
  expect(canonical(html)).toBe(`${SITE}/guide/quickstart`)
  expect(meta(html, 'property', 'og:url')).toBe(`${SITE}/guide/quickstart`)
  expect(meta(html, 'property', 'og:title')).toBe('Quickstart | CLM guide')
  expect(meta(html, 'property', 'og:type')).toBe('website')
  expect(meta(html, 'property', 'og:site_name')).toBe('CLM')
  expect(meta(html, 'name', 'twitter:card')).toBe('summary_large_image')
  const image = meta(html, 'property', 'og:image')!
  expect(image).toMatch(new RegExp(`^${SITE}/.+\\.png$`))
  const png = readFileSync(join(mainDist, image.slice(SITE.length)))
  expect(png.subarray(1, 4).toString()).toBe('PNG')
  expect([png.readUInt32BE(16), png.readUInt32BE(20)]).toEqual([1200, 630])
  expect(html).toContain(`<link rel="alternate" type="text/markdown" href="${SITE}/guide/quickstart.md">`)
})

test('a summary paragraph loses its Markdown in the description', () => {
  test.setTimeout(180_000)
  // req:web.site/page-metadata@6cc8ba05
  const dir = fixture('plain', {
    'README.md': '# Guide\n\nThe guide.\n\n1. [Why CLM](why-clm.md)\n',
    'why-clm.md': '# Why\n\nRun `clm guide` or read [Why CLM](why-clm.md) and **more**.\n',
  })
  const { out, error } = build('plain', { GUIDE_DIR: dir })
  expect(error).toBeUndefined()
  expect(meta(read(out, 'guide/why-clm.html'), 'name', 'description')).toBe(
    'Run clm guide or read Why CLM and more.',
  )
})

test('a guide file with no summary paragraph fails the build, naming it', () => {
  test.setTimeout(180_000)
  // req:web.site/page-metadata@6cc8ba05
  const dir = fixture('nosummary', {
    'README.md': '# Guide\n\nThe guide.\n',
    'broken.md': '# Broken\n\n- a list\n- and no paragraph\n',
  })
  const { error } = build('nosummary', { GUIDE_DIR: dir })
  expect(error).toBeDefined()
  expect(error).toContain('broken.md')
})

test('the homepage, privacy and 404 pages have their own heads', () => {
  // req:web.site/page-metadata@6cc8ba05
  const home = read(mainDist, 'index.html')
  const privacy = read(mainDist, 'privacy.html')
  expect(canonical(home)).toBe(`${SITE}/`)
  expect(canonical(privacy)).toBe(`${SITE}/privacy`)
  expect(meta(home, 'name', 'description')!.length).toBeGreaterThan(0)
  expect(meta(privacy, 'name', 'description')!.length).toBeGreaterThan(0)
  const titles = filesUnder(mainDist)
    .filter((f) => f.endsWith('.html') && f !== '404.html')
    .map((f) => title(read(mainDist, f)))
  expect(new Set(titles).size).toBe(titles.length)
  expect(titles).toContain('CLM guide')
  const notFound = read(mainDist, '404.html')
  expect(title(notFound)).toBe('Page not found | CLM')
  expect(notFound).toContain('<meta name="robots" content="noindex">')
  expect(canonical(notFound)).toBeUndefined()
})

test('robots.txt has one group for every crawler and names the sitemap', () => {
  // req:web.site/crawler-files@4c151612
  const lines = read(mainDist, 'robots.txt').split('\n').map((l) => l.trim()).filter(Boolean)
  expect(lines.filter((l) => /^user-agent:/i.test(l))).toEqual(['User-agent: *'])
  expect(lines.filter((l) => /^allow:/i.test(l))).toEqual([])
  expect(lines.filter((l) => /^disallow:/i.test(l))).toEqual([
    'Disallow: /app/',
    'Disallow: /api/',
    'Disallow: /auth/',
  ])
  expect(lines).toContain(`Sitemap: ${SITE}/sitemap.xml`)
})

test('the sitemap lists the pages and nothing under /app/', () => {
  // req:web.site/crawler-files@4c151612
  const urls = Array.from(read(mainDist, 'sitemap.xml').matchAll(/<loc>([^<]+)<\/loc>/g), (m) => m[1])
  for (const path of ['/', '/guide/quickstart', '/guide/spec-format', '/privacy']) {
    expect(urls).toContain(SITE + path)
  }
  const guides = readdirSync(join(repoDocs, 'guide')).filter((f) => f.endsWith('.md') && f !== 'README.md')
  for (const f of guides) expect(urls).toContain(`${SITE}/guide/${f.slice(0, -3)}`)
  expect(urls.filter((u) => u.includes('/app/') || u.includes('404'))).toEqual([])
})

test('the homepage describes CLM as a SoftwareApplication', () => {
  // req:web.site/structured-data@25bd0bd6
  const [app, ...rest] = ldJson(read(mainDist, 'index.html'))
  expect(rest).toEqual([])
  expect(app['@type']).toBe('SoftwareApplication')
  expect(app.name).toBe('CLM')
  expect(app.url).toBe(`${SITE}/`)
  expect(app.offers.price).toBe('0')
  expect(app.license).toContain('MIT')
  expect(app.sameAs).toContain('https://github.com/' + 'clunny' + '/clm')
})

test("the FAQ's structured data follows its file", () => {
  test.setTimeout(180_000)
  // req:web.site/structured-data@25bd0bd6
  const dir = fixture('faq', {
    'README.md': '# Guide\n\nThe guide.\n\n1. [FAQ](faq.md)\n',
    'faq.md':
      '# FAQ\n\nQuestions.\n\n## What is CLM?\n\nA tool for **building** software with `AI`.\n\n## Is it free?\n\nYes, see [the licence](https://example.com/licence).\n',
  })
  const { out, error } = build('faq', { GUIDE_DIR: dir })
  expect(error).toBeUndefined()
  const [faq] = ldJson(read(out, 'guide/faq.html'))
  expect(faq['@type']).toBe('FAQPage')
  expect(faq.mainEntity.map((q: { name: string }) => q.name)).toEqual(['What is CLM?', 'Is it free?'])
  expect(faq.mainEntity.map((q: { acceptedAnswer: { text: string } }) => q.acceptedAnswer.text)).toEqual([
    'A tool for building software with AI.',
    'Yes, see the licence.',
  ])
  // A guide with no `faq.md` has no FAQ data, and still builds.
  expect(ldJson(read(out, 'guide/index.html'))).toEqual([])
})

test("each guide file's Markdown copy is its text with the links rewritten", () => {
  // req:web.site/agent-markdown@a56b453f
  const pages = guideSidebar()
  expect(pages.length).toBeGreaterThan(3)
  for (const name of readdirSync(join(repoDocs, 'guide')).filter((f) => f.endsWith('.md'))) {
    const page = name === 'README.md' ? 'index' : name.slice(0, -3)
    const file = join(repoDocs, 'guide', name)
    const copy = read(mainDist, `guide/${page}.md`)
    expect(copy, name).toContain(readFileSync(file, 'utf8').split('\n')[0])
    // No link to another guide file or into the repository stays relative.
    expect(copy.replace(/```[\s\S]*?```/g, ''), name).not.toMatch(/\]\((\.\.?\/|[\w-]+\.md)/)
  }
  expect(read(mainDist, 'guide/spec-format.md').split('\n')[0]).toBe('# Spec format (v0, provisional)')
})

test('links in a Markdown copy are absolute, and code is left alone', () => {
  test.setTimeout(180_000)
  // req:web.site/agent-markdown@a56b453f
  const dir = fixture('copies', {
    'README.md': '# Guide\n\nThe guide.\n\n1. [Reference](reference.md)\n',
    'reference.md':
      '# Reference\n\nSee the [spec format](../spec-format.md#top), the [guide](README.md),\n\n```\n[not a link](../spec-format.md)\n```\n',
  })
  const { out, error } = build('copies', { GUIDE_DIR: dir })
  expect(error).toBeUndefined()
  const copy = read(out, 'guide/reference.md')
  expect(copy).toContain(`[spec format](${SITE}/guide/spec-format.md#top)`)
  expect(copy).toContain(`[guide](${SITE}/guide/index.md)`)
  expect(copy).toContain('[not a link](../spec-format.md)')
  // A link into the repository names the file on GitHub's `main`, a directory by `tree`.
  const page = { name: 'reference', file: '', title: '', summary: '' }
  const repo = 'someone/clm-public'
  expect(rewriteLinks('[c](../../crates/) [r](../../README.md#top) [x](https://example.com/a.md)', page, SITE, repo)).toBe(
    `[c](https://github.com/${repo}/tree/main/crates) [r](https://github.com/${repo}/blob/main/README.md#top) [x](https://example.com/a.md)`,
  )
})

test('llms.txt lists the copies in the sidebar order', () => {
  // req:web.site/agent-markdown@a56b453f
  const text = read(mainDist, 'llms.txt')
  expect(text.startsWith('# CLM\n\n> ')).toBe(true)
  expect(text).toContain('/plugin marketplace add ' + 'clunny' + '/clm')
  const why = text.indexOf(`${SITE}/guide/why-clm.md`)
  const quickstart = text.indexOf(`${SITE}/guide/quickstart.md`)
  expect(why).toBeGreaterThan(0)
  expect(why).toBeLessThan(quickstart)
  const summary = 'From nothing to your first change built, for a new project or an existing one.'
  expect(text).toContain(`[Quickstart](${SITE}/guide/quickstart.md): ${summary}`)
  expect(text).toContain(`${SITE}/llms-full.txt`)
})

test('llms-full.txt and llms.txt follow the guide files with no edit under web/site', () => {
  test.setTimeout(240_000)
  // req:web.site/agent-markdown@a56b453f
  const dir = fixture('drift', {
    'README.md': '# Guide\n\nThe guide.\n\n1. [Why](why.md)\n',
    'why.md': '# Why\n\nBecause.\n',
  })
  const first = build('drift', { GUIDE_DIR: dir })
  expect(first.error).toBeUndefined()
  expect(read(first.out, 'llms-full.txt')).not.toContain('A line added later.')
  const fixtureWrite = (file: string, text: string) => writeFileSync(join(dir, file), text)
  fixtureWrite('why.md', '# Why\n\nBecause.\n\nA line added later.\n')
  fixtureWrite('extra.md', '# Extra\n\nA new file.\n')
  fixtureWrite('README.md', '# Guide\n\nThe guide.\n\n1. [Why](why.md)\n2. [Extra](extra.md)\n')
  const second = build('drift', { GUIDE_DIR: dir })
  expect(second.error).toBeUndefined()
  const full = read(second.out, 'llms-full.txt')
  expect(full).toContain('A line added later.')
  expect(full).toContain('A new file.')
  expect(full).toContain(`<!-- ${SITE}/guide/extra -->`)
  expect(read(second.out, 'llms.txt')).toContain(`${SITE}/guide/extra.md`)
})

test('no built agent file is committed', () => {
  // req:web.site/agent-markdown@a56b453f
  const tracked = execFileSync('git', ['ls-files'], { cwd: resolve(siteDir, '../..'), encoding: 'utf8' }).split('\n')
  expect(tracked.filter((f) => /(^|\/)llms(-full)?\.txt$/.test(f))).toEqual([])
  expect(tracked.filter((f) => f.startsWith('web/site/public/') && f.endsWith('.md'))).toEqual([])
  expect(existsSync(join(siteDir, 'public'))).toBe(true)
})

test.describe('with script disabled', () => {
  test.use({ javaScriptEnabled: false })

  test('the static HTML holds the homepage and the quickstart', async ({ page }) => {
    // req:web.site/readable-without-script@5e50e6d9
    await page.goto('/')
    await expect(page.locator('h1')).toContainText('Declare the intent.')
    await expect(page.locator('h1')).toContainText('CLM keeps it matching.')
    const headings = await page.locator('.clm-home .sec .k').allTextContents()
    expect(headings.map((h) => h.trim())).toEqual([
      'The split',
      'The loop',
      'Not a vibe-coded mess',
      'Solo or as a team',
      'Standing on shoulders',
    ])
    await expect(page.locator('.cmd')).toContainText('/plugin marketplace add ' + 'clunny' + '/clm')
    await expect(page.locator('a[href="https://github.com/' + 'clunny' + '/clm"]').first()).toBeVisible()
    await page.goto('/guide/quickstart')
    await expect(page.locator('h1')).toContainText('Quickstart')
    await expect(page.locator('.vp-doc')).toContainText('/plugin marketplace add ' + 'clunny' + '/clm')
  })
})

test('the footer of the homepage and of a guide page leads to the privacy page', async ({ page }) => {
  // req:web.site/privacy-page@7d20639a
  for (const path of ['/', '/guide/quickstart']) {
    await page.goto(path)
    await page.locator('footer a[href="/privacy"]').click()
    await expect(page).toHaveURL(/\/privacy$/)
    for (const word of ['cookies', 'Umami', 'Do Not Track', 'IP address', '/app/']) {
      await expect(page.locator('main')).toContainText(word)
    }
  }
})
