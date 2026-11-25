// The site is built from this repository's guide, so an edit to it selects this file
// (`doc-readers-marked`):
// clm:reads docs/guide/**
// clm:reads docs/spec-format.md
import { test, expect } from '@playwright/test'
import { readFileSync, readdirSync, writeFileSync } from 'node:fs'
import { join } from 'node:path'
import { blogFixture, build, filesUnder, fixture, mainDist, post, read, repoDocs, serve } from './helpers'

const SITE = 'https://clm-app.dev'
const meta = (html: string, attr: 'name' | 'property', key: string) =>
  new RegExp(`<meta ${attr}="${key}" content="([^"]*)"`).exec(html)?.[1]
const title = (html: string) => /<title>([^<]*)<\/title>/.exec(html)?.[1]
const canonical = (html: string) => /<link rel="canonical" href="([^"]*)"/.exec(html)?.[1]
const ldJson = (html: string) =>
  Array.from(html.matchAll(/<script type="application\/ld\+json">([^<]*)<\/script>/g), (m) => JSON.parse(m[1]))

/** A guide file's `#` title, and its first paragraph after it as one line, read here from the
 * file rather than pinned, so a reworded page doesn't fail the test. The quickstart's holds no
 * Markdown marks, so the line is the plain text the head should carry. */
function titleAndFirstParagraph(file: string): { heading: string; sentence: string } {
  const lines = readFileSync(file, 'utf8').split('\n')
  const at = lines.findIndex((l) => l.startsWith('# '))
  let i = at + 1
  while (lines[i].trim() === '') i++
  const para: string[] = []
  for (; i < lines.length && lines[i].trim() !== ''; i++) para.push(lines[i].trim())
  const sentence = para.join(' ')
  expect(sentence, 'the first paragraph holds no Markdown').not.toMatch(/[`*_[\]]/)
  return { heading: lines[at].slice(2).trim(), sentence }
}

test("a guide page's head holds its metadata and its social card", () => {
  // req:web.site/page-metadata@8d1c2456
  const html = read(mainDist, 'guide/quickstart.html')
  const { heading, sentence } = titleAndFirstParagraph(join(repoDocs, 'guide/quickstart.md'))
  expect(title(html)).toBe(`${heading} | CLM guide`)
  expect(meta(html, 'name', 'description')).toBe(sentence)
  expect(meta(html, 'property', 'og:description')).toBe(sentence)
  expect(meta(html, 'name', 'twitter:description')).toBe(sentence)
  expect(canonical(html)).toBe(`${SITE}/guide/quickstart`)
  expect(meta(html, 'property', 'og:url')).toBe(`${SITE}/guide/quickstart`)
  expect(meta(html, 'property', 'og:title')).toBe(`${heading} | CLM guide`)
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
  // req:web.site/page-metadata@8d1c2456
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
  // req:web.site/page-metadata@8d1c2456
  const dir = fixture('nosummary', {
    'README.md': '# Guide\n\nThe guide.\n',
    'broken.md': '# Broken\n\n- a list\n- and no paragraph\n',
  })
  const { error } = build('nosummary', { GUIDE_DIR: dir })
  expect(error).toBeDefined()
  expect(error).toContain('broken.md')
})

test('the homepage, privacy and 404 pages have their own heads', () => {
  // req:web.site/page-metadata@8d1c2456
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

test('a guide file with no # title fails the build, naming it', () => {
  test.setTimeout(180_000)
  // req:web.site/page-metadata@8d1c2456
  const dir = fixture('notitle', {
    'README.md': '# Guide\n\nThe guide.\n',
    'untitled.md': 'A page with no title line.\n\n## A section\n\nText.\n',
  })
  const { error } = build('notitle', { GUIDE_DIR: dir })
  expect(error).toBeDefined()
  expect(error).toContain('untitled.md')
})

test("each page has its own title and description, and the 404 page its own in the browser too", async ({ page }) => {
  // req:web.site/page-metadata@8d1c2456
  await page.goto('/')
  const first = await page.evaluate(() => ({
    headline: (document.querySelector('.hero h1') as HTMLElement).innerText,
    line: (document.querySelector('.hero p') as HTMLElement).innerText,
  }))
  const flat = (s: string) => s.replace(/\s+/g, ' ').trim()
  const sentence = /^.*?\.(?=\s|$)/.exec(flat(first.line))![0]
  const home = read(mainDist, 'index.html')
  expect(title(home)).toBe('CLM: declare the intent, AI builds the code to match')
  expect(meta(home, 'name', 'description')).toBe(`${flat(first.headline)} ${sentence}`)
  expect(title(read(mainDist, 'guide/index.html'))).toBe('CLM guide')
  expect(title(read(mainDist, 'privacy.html'))).toBe('Privacy | CLM')
  expect(title(read(mainDist, 'blog/index.html'))).toBe('CLM blog')
  const notFound = read(mainDist, '404.html')
  expect(title(notFound)).toBe('Page not found | CLM')
  expect(Array.from(notFound.matchAll(/<meta name="description" content="([^"]*)"/g), (m) => m[1])).toEqual([
    'This page does not exist.',
  ])
  expect(notFound).not.toMatch(/<meta (property|name)="(og|twitter):/)
  await page.goto('/no-such-page')
  await expect(page).toHaveTitle('Page not found | CLM')
})

test('robots.txt has one group for every crawler and names the sitemap', () => {
  // req:web.site/crawler-files@08777c25
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
  // req:web.site/crawler-files@08777c25
  const urls = Array.from(read(mainDist, 'sitemap.xml').matchAll(/<loc>([^<]+)<\/loc>/g), (m) => m[1])
  for (const path of ['/', '/guide/', '/guide/quickstart', '/guide/spec-format', '/privacy']) {
    expect(urls).toContain(SITE + path)
  }
  const guides = readdirSync(join(repoDocs, 'guide')).filter((f) => f.endsWith('.md') && f !== 'README.md')
  for (const f of guides) expect(urls).toContain(`${SITE}/guide/${f.slice(0, -3)}`)
  expect(urls.filter((u) => u.includes('/app/') || u.includes('404'))).toEqual([])
})

test('the homepage describes CLM as a SoftwareApplication', () => {
  // req:web.site/structured-data@7aab789a
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
  // req:web.site/structured-data@7aab789a
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
  expect(ldJson(read(out, 'guide/index.html'))).toEqual([])
})

test('a guide with no FAQ, or an FAQ with no question, builds with no FAQPage anywhere', () => {
  test.setTimeout(180_000)
  // req:web.site/structured-data@7aab789a
  const guides = {
    'no faq.md': fixture('nofaq', { 'README.md': '# Guide\n\nThe guide.\n' }),
    'no ## heading': fixture('noquestions', {
      'README.md': '# Guide\n\nThe guide.\n\n1. [FAQ](faq.md)\n',
      'faq.md': '# FAQ\n\nNo questions yet.\n\n### Not a question\n\nText.\n',
    }),
  }
  for (const [label, dir] of Object.entries(guides)) {
    const { out, error } = build('noquestions', { GUIDE_DIR: dir })
    expect(error, label).toBeUndefined()
    const pages = filesUnder(out).filter((f) => f.endsWith('.html'))
    expect(pages, label).toContain('guide/index.html')
    expect(pages.filter((f) => read(out, f).includes('FAQPage')), label).toEqual([])
  }
})

/** A line with each inline link's target, outside code spans, made `<>`: what a copy may change. */
const targetsHidden = (line: string) =>
  line
    .split(/(`[^`]*`)/)
    .map((piece, i) => (i % 2 ? piece : piece.replace(/(?<!!)(\[[^\]]*\]\()[^)\s]+\)/g, '$1<>)')))
    .join('')

/** The lines of `text` outside fenced code, code spans taken out. */
const proseLines = (text: string) => {
  let fenced = false
  return text.split('\n').filter((line) => {
    if (/^\s*(```|~~~)/.test(line)) fenced = !fenced
    else if (!fenced) return true
    return false
  }).map((line) => line.replace(/`[^`]*`/g, ''))
}

test("each guide file's Markdown copy is its file with only link targets changed", () => {
  // req:web.site/agent-markdown@e6e9273a
  const files = readdirSync(join(repoDocs, 'guide')).filter((f) => f.endsWith('.md'))
  const copies: string[] = []
  for (const [file, name] of [
    ...files.map((f) => [join(repoDocs, 'guide', f), f === 'README.md' ? 'index' : f.slice(0, -3)]),
    [join(repoDocs, 'spec-format.md'), 'spec-format'],
  ]) {
    copies.push(`${name}.md`)
    const source = readFileSync(file, 'utf8').split('\n')
    const copy = read(mainDist, `guide/${name}.md`).split('\n')
    expect(copy.length, name).toBe(source.length)
    for (let i = 0; i < copy.length; i++) expect(targetsHidden(copy[i]), `${name}:${i + 1}`).toBe(targetsHidden(source[i]))
    // No inline link outside fenced code is left relative.
    for (const line of proseLines(copy.join('\n'))) {
      for (const m of line.matchAll(/(?<!!)\[[^\]]*\]\(([^)\s]+)\)/g)) {
        expect(m[1], `${name}: ${line}`).toMatch(/^([a-z][a-z0-9+.-]*:|#|\/)/i)
      }
    }
  }
  expect(read(mainDist, 'guide/reference.md')).toContain(`](${SITE}/guide/spec-format.md)`)
  const built = filesUnder(join(mainDist, 'guide')).filter((f) => f.endsWith('.md'))
  expect(built.sort()).toEqual(copies.sort())
})

test('links into the repository name the file on GitHub in the copies and on the pages', async ({ page }) => {
  test.setTimeout(180_000)
  // req:web.site/agent-markdown@e6e9273a
  const repo = 'someone/clm-public'
  const dir = fixture('repolinks', {
    'README.md': '# Guide\n\nThe guide.\n\n1. [Reference](reference.md)\n',
    'reference.md': [
      '# Reference',
      '',
      'See the [spec format](../spec-format.md#top), the [guide](README.md), the [crates](../../crates/),',
      'the [readme](../../README.md#top), the [privacy page](/privacy) and [elsewhere](https://example.com/a.md).',
      '',
      '```',
      '[not a link](../spec-format.md)',
      '```',
      '',
      'And `[x](../y.md)` as code.',
      '',
    ].join('\n'),
  })
  const blog = blogFixture('repolinks', {
    'alpha.md': post(
      { title: 'Alpha', summary: 'First.', date: '2026-11-05', draft: false },
      'The [readme](../../../README.md#top), the [next part](next.md) and ![pic](next.md).',
    ),
    'next.md': post({ title: 'Next', summary: 'Later.', date: '2026-11-06', draft: true }, 'Soon.'),
  })
  const { out, error } = build('repolinks', { GUIDE_DIR: dir, BLOG_DIR: blog, INSTALL_REPO: repo })
  expect(error).toBeUndefined()
  const tree = `https://github.com/${repo}/tree/main/crates`
  const readme = `https://github.com/${repo}/blob/main/README.md#top`
  const copy = read(out, 'guide/reference.md')
  for (const want of [
    `[spec format](${SITE}/guide/spec-format.md#top)`,
    `[guide](${SITE}/guide/index.md)`,
    `[crates](${tree})`,
    `[readme](${readme})`,
    '[privacy page](/privacy)',
    '[elsewhere](https://example.com/a.md)',
    '[not a link](../spec-format.md)',
    '`[x](../y.md)`',
  ]) {
    expect(copy).toContain(want)
  }
  const server = await serve(out)
  try {
    await page.goto(`${server.url}/guide/reference`)
    const hrefs = await page.locator('.vp-doc a').evaluateAll((as) => as.map((a) => a.getAttribute('href')))
    expect(hrefs).toEqual(expect.arrayContaining([tree, readme]))
    await page.goto(`${server.url}/blog/alpha`)
    await expect(page.locator('.post-body a', { hasText: 'readme' })).toHaveAttribute('href', readme)
  } finally {
    await server.close()
  }
  const postCopy = read(out, 'blog/alpha.md')
  expect(postCopy).toContain(`[readme](${readme})`)
  expect(postCopy).toContain('the next part and ![pic](next.md).')
})

test('llms.txt lists the copies in the sidebar order', () => {
  // req:web.site/agent-markdown@e6e9273a
  const text = read(mainDist, 'llms.txt')
  expect(text.startsWith('# CLM\n\n> ')).toBe(true)
  expect(text).toContain('/plugin marketplace add ' + 'clunny' + '/clm')
  const why = text.indexOf(`${SITE}/guide/why-clm.md`)
  const quickstart = text.indexOf(`${SITE}/guide/quickstart.md`)
  expect(why).toBeGreaterThan(0)
  expect(why).toBeLessThan(quickstart)
  const { heading, sentence } = titleAndFirstParagraph(join(repoDocs, 'guide/quickstart.md'))
  expect(text).toContain(`[${heading}](${SITE}/guide/quickstart.md): ${sentence}`)
  expect(text).toContain(`${SITE}/llms-full.txt`)
})

test('llms-full.txt and llms.txt follow the guide files with no edit under web/site', () => {
  test.setTimeout(240_000)
  // req:web.site/agent-markdown@e6e9273a
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

test.describe('with script disabled', () => {
  test.use({ javaScriptEnabled: false })

  test('the static HTML holds the homepage and the quickstart', async ({ page }) => {
    // req:web.site/readable-without-script@315b4c02
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
    const file = join(repoDocs, 'guide/quickstart.md')
    await expect(page.locator('h1')).toContainText(titleAndFirstParagraph(file).heading)
    // The text of the file's first code block.
    const lines = readFileSync(file, 'utf8').split('\n')
    const open = lines.findIndex((l) => /^\s*(```|~~~)/.test(l))
    const close = lines.findIndex((l, i) => i > open && /^\s*(```|~~~)/.test(l))
    const code = lines.slice(open + 1, close).filter((l) => l.trim() !== '')
    expect(code.length).toBeGreaterThan(0)
    const shown = await page.locator('.vp-doc div[class*="language-"] pre').first().innerText()
    expect(shown.split('\n').map((l) => l.trimEnd()).filter((l) => l.trim() !== '')).toEqual(
      code.map((l) => l.trimEnd()),
    )
  })
})

test('the footer of the homepage and of a guide page leads to the privacy page', async ({ page }) => {
  // req:web.site/privacy-page@a765e60a
  for (const path of ['/', '/guide/quickstart']) {
    await page.goto(path)
    await expect(page.locator('.VPNav a[href="/privacy"]'), path).toHaveCount(0)
    await expect(page.locator('.VPSidebar a[href="/privacy"]'), path).toHaveCount(0)
    if (path !== '/') await expect(page.locator('.VPSidebar a').first()).toBeVisible()
    await page.locator('footer a[href="/privacy"]').click()
    await expect(page).toHaveURL(/\/privacy$/)
    for (const word of ['cookies', 'Umami', 'Do Not Track', 'IP address', '/app/']) {
      await expect(page.locator('main')).toContainText(word)
    }
  }
})
