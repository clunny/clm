import { test, expect } from '@playwright/test'
import { execFileSync } from 'node:child_process'
import { existsSync, mkdirSync, readFileSync, readdirSync, rmSync, writeFileSync } from 'node:fs'
import { join, resolve } from 'node:path'

const siteDir = resolve(import.meta.dirname, '..')
const repoDocs = resolve(siteDir, '../../docs')

// Builds the site from a guide directory into `.test-dist/<name>`; returns the output or the failure.
function build(name: string, guideDir: string): { out: string; error?: string } {
  const out = join(siteDir, '.test-dist', name)
  rmSync(out, { recursive: true, force: true })
  try {
    execFileSync('npx', ['vitepress', 'build', '.'], {
      cwd: siteDir,
      env: { ...process.env, SITE_OUT: out, GUIDE_DIR: guideDir },
      stdio: 'pipe',
      maxBuffer: 64 * 1024 * 1024,
    })
    return { out }
  } catch (e) {
    const err = e as { stdout?: Buffer; stderr?: Buffer; message: string }
    return { out, error: `${err.stdout ?? ''}${err.stderr ?? ''}${err.message}` }
  }
}

// A fixture guide under `.test-dist/<name>-src/docs/guide`, with its sibling `spec-format.md`.
function fixture(name: string, files: Record<string, string>): string {
  const docs = join(siteDir, '.test-dist', `${name}-src`, 'docs')
  rmSync(docs, { recursive: true, force: true })
  mkdirSync(join(docs, 'guide'), { recursive: true })
  writeFileSync(join(docs, 'spec-format.md'), '# Spec format\n\nThe format.\n')
  for (const [file, text] of Object.entries(files)) writeFileSync(join(docs, 'guide', file), text)
  return join(docs, 'guide')
}

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
    'README.md': '# Guide\n\n1. [Why](why-clm.md)\n',
    'why-clm.md': '# Why\n',
    'brand-new.md': '# Brand new\n\nA page nobody listed.\n',
  })
  const { out, error } = build('added', dir)
  expect(error).toBeUndefined()
  expect(readFileSync(join(out, 'guide/brand-new.html'), 'utf8')).toContain('A page nobody listed.')
})

test('links between guide files lead to their pages', () => {
  test.setTimeout(180_000)
  // req:web.site/guide-pages@846e6608
  const dir = fixture('links', {
    'README.md': '# Guide\n\n1. [Why CLM](why-clm.md)\n2. [Reference](reference.md)\n',
    'why-clm.md': '# Why\n\nBack to the [guide](README.md).\n',
    'reference.md': '# Reference\n\nSee the [spec format](../spec-format.md).\n',
  })
  const { out, error } = build('links', dir)
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
  const { error } = build('dead', dir)
  expect(error).toBeDefined()
  expect(error).toMatch(/dead link/i)
  expect(error).toContain('missing')
})
