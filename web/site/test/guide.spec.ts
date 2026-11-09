// The site is built from this repository's guide, so an edit to it selects this file
// (`doc-readers-marked`):
// clm:reads docs/guide/**
// clm:reads docs/spec-format.md
import { test, expect } from '@playwright/test'
import { existsSync, readFileSync, readdirSync } from 'node:fs'
import { join } from 'node:path'
import { build, fixture, repoDocs, siteDir } from './helpers'

// The links written in a page's own text, resolved against its address: the sidebar, nav and pager
// carry a class, the text's links don't.
const bodyLinks = (html: string, path: string) =>
  Array.from(html.matchAll(/<a href="([^"#]+)"/g), (m) => new URL(m[1], `http://site${path}`).pathname)

test('every guide file has a page and the sidebar follows the README', async ({ page }) => {
  // req:web.site/guide-pages@846e6608
  const files = readdirSync(join(repoDocs, 'guide')).filter((f) => f.endsWith('.md'))
  expect(files).toContain('README.md')
  const dist = join(siteDir, '.test-dist', 'main')
  for (const f of files) {
    const route = f === 'README.md' ? 'guide/index.html' : `guide/${f.slice(0, -3)}.html`
    expect(existsSync(join(dist, route)), route).toBe(true)
  }
  expect(existsSync(join(dist, 'guide/spec-format.html'))).toBe(true)
  const readmeTitle = /^# (.+)$/m.exec(readFileSync(join(repoDocs, 'guide/README.md'), 'utf8'))![1]
  expect(readFileSync(join(dist, 'guide/index.html'), 'utf8')).toContain(readmeTitle)

  await page.goto('/guide/quickstart')
  const labels = await page.locator('.VPSidebar a').allInnerTexts()
  expect(labels.indexOf('Why CLM')).toBeGreaterThanOrEqual(0)
  expect(labels.indexOf('Why CLM')).toBeLessThan(labels.indexOf('Quickstart'))
  expect(labels[labels.length - 1]).toBe('Spec format')
})

test('a file added to the guide gets a page with no edit under web/site', () => {
  test.setTimeout(180_000)
  // req:web.site/guide-pages@846e6608
  const dir = fixture('added', {
    'README.md': '# Guide\n\nThe guide.\n\n1. [Why](why-clm.md)\n',
    'why-clm.md': '# Why\n\nWhy CLM.\n',
    'brand-new.md': '# Brand new\n\nA page nobody listed.\n',
  })
  const { out, error } = build('added', { GUIDE_DIR: dir })
  expect(error).toBeUndefined()
  expect(readFileSync(join(out, 'guide/brand-new.html'), 'utf8')).toContain('A page nobody listed.')
})

test('links between guide files lead to their pages', () => {
  test.setTimeout(180_000)
  // req:web.site/guide-pages@846e6608
  const dir = fixture('links', {
    'README.md': '# Guide\n\nThe guide.\n\n1. [Why CLM](why-clm.md)\n2. [Reference](reference.md)\n',
    'why-clm.md': '# Why\n\nBack to the [guide](README.md).\n',
    'reference.md': '# Reference\n\nSee the [spec format](../spec-format.md).\n',
  })
  const { out, error } = build('links', { GUIDE_DIR: dir })
  expect(error).toBeUndefined()
  const links = (name: string, path: string) =>
    bodyLinks(readFileSync(join(out, `guide/${name}.html`), 'utf8'), path)
  expect(links('index', '/guide/')).toContain('/guide/why-clm')
  expect(links('reference', '/guide/reference')).toContain('/guide/spec-format')
  expect(links('why-clm', '/guide/why-clm')).toContain('/guide/')
  expect(existsSync(join(out, 'guide/why-clm.html'))).toBe(true)
  expect(existsSync(join(out, 'guide/spec-format.html'))).toBe(true)
})

test('a dead link fails the build, naming it', () => {
  test.setTimeout(180_000)
  // req:web.site/guide-pages@846e6608
  const dir = fixture('dead', {
    'README.md': '# Guide\n\nSee [nowhere](missing.md).\n',
  })
  const { error } = build('dead', { GUIDE_DIR: dir })
  expect(error).toBeDefined()
  expect(error).toMatch(/dead link/i)
  expect(error).toContain('missing')
})
