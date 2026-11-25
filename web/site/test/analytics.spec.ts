// The site is built from this repository's guide, so an edit to it selects this file
// (`doc-readers-marked`):
// clm:reads docs/guide/**
// clm:reads docs/spec-format.md
import { test, expect, type Page } from '@playwright/test'
import { readFileSync } from 'node:fs'
import { join } from 'node:path'
import { build, filesUnder, mainDist, read, repoDocs } from './helpers'

// The analytics global's name is built from pieces so that no file of this suite names it whole
// either; it is the one the service's script defines.
const service = ['u', 'mam', 'i'].join('')

/** Stubs the service before the page's own script runs, recording each `track` call. */
async function stub(page: Page) {
  await page.addInitScript((name) => {
    const calls: unknown[][] = []
    ;(window as unknown as Record<string, unknown>)[name] = { track: (...args: unknown[]) => calls.push(args) }
    ;(window as unknown as { calls: unknown[][] }).calls = calls
  }, service)
}
const calls = (page: Page) => page.evaluate(() => (window as unknown as { calls: unknown[][] }).calls)

test('a released build loads the analytics script on every page', () => {
  test.setTimeout(180_000)
  // req:web.site/analytics-script@d3f71ebe
  const { out, error } = build('analytics-on', { UMAMI_WEBSITE_ID: 'test-id' })
  expect(error).toBeUndefined()
  for (const file of ['index.html', 'guide/quickstart.html', 'privacy.html', '404.html']) {
    const tags = read(out, file).match(/<script[^>]*cloud\.umami\.is\/script\.js[^>]*>/g) ?? []
    expect(tags, file).toHaveLength(1)
    expect(tags[0], file).toContain('data-website-id="test-id"')
    expect(tags[0], file).toContain('data-do-not-track="true"')
    expect(tags[0], file).toContain('defer')
  }
})

test('a build with no website ID names the service nowhere', () => {
  // req:web.site/analytics-script@d3f71ebe
  const named = filesUnder(mainDist).filter((f) => read(mainDist, f).includes(service))
  expect(named).toEqual([])
})

test('Get started reports a click and still goes on', async ({ page }) => {
  // req:web.site/click-events@4180dc69
  await stub(page)
  await page.goto('/')
  await page.locator('.hero a.btn.p').click()
  await expect(page).toHaveURL(/\/guide\/quickstart$/)
  // The in-site navigation keeps the page, and the stub with it.
  expect(await calls(page)).toEqual([['click', { label: 'Get started', path: '/' }]])
})

test('How it works and a nav link report clicks with their own text', async ({ page }) => {
  // req:web.site/click-events@4180dc69
  await stub(page)
  await page.goto('/')
  await page.locator('.hero a.btn.s').click()
  await page.locator('.VPNavBarMenu a', { hasText: 'Guide' }).click()
  expect(await calls(page)).toEqual([
    ['click', { label: 'How it works', path: '/' }],
    ['click', { label: 'Guide', path: '/' }],
  ])
})

test('the nav Web app reports a click with its own text', async ({ page }) => {
  // req:web.site/click-events@4180dc69
  await stub(page)
  await page.goto('/')
  await page.evaluate(() => document.addEventListener('click', (e) => e.preventDefault()))
  await page.locator('.VPNavBarMenu a', { hasText: 'Web app' }).click()
  expect(await calls(page)).toEqual([['click', { label: 'Web app', path: '/' }]])
})

test('a link to another host reports an outbound click only', async ({ page }) => {
  // req:web.site/click-events@4180dc69
  await stub(page)
  await page.goto('/')
  // Navigation held: the click is reported, and the page stays to be read.
  await page.evaluate(() => document.addEventListener('click', (e) => e.preventDefault()))
  await page.locator('a[href="https://github.com/obra/superpowers"]').click()
  expect(await calls(page)).toEqual([
    ['outbound', { url: 'https://github.com/obra/superpowers', path: '/' }],
  ])
})

test('clicks work with no analytics service, one with no track, or a track that throws', async ({ browser }) => {
  // req:web.site/click-events@4180dc69
  const services: Record<string, string> = {
    'no service': '',
    'no track': `window.${service} = {}`,
    'a track that throws': `window.${service} = { track: () => { throw new Error('blocked') } }`,
  }
  for (const [label, script] of Object.entries(services)) {
    const page = await browser.newPage()
    try {
      const errors: string[] = []
      page.on('pageerror', (e) => errors.push(e.message))
      page.on('console', (m) => m.type() === 'error' && errors.push(m.text()))
      if (script) await page.addInitScript(script)
      await page.goto('/')
      await page.locator('.hero a.btn.p').click()
      await expect(page, label).toHaveURL(/\/guide\/quickstart$/)
      expect(errors, label).toEqual([])
    } finally {
      await page.close()
    }
  }
})

test('a sidebar link, the footer Guide link and a mailto: link report nothing', async ({ page }) => {
  // req:web.site/click-events@4180dc69
  await stub(page)
  // Navigation held: a listener ahead of VitePress's router, which takes clicks on the window as
  // they go down, so neither it nor the browser follows a link.
  await page.addInitScript(() => window.addEventListener('click', (e) => e.preventDefault(), { capture: true }))
  await page.goto('/guide/quickstart')
  await page.locator('.VPSidebar a').first().click()
  await expect(page).toHaveURL(/\/guide\/quickstart$/)
  await page.goto('/')
  await page.evaluate(() => {
    const mail = document.createElement('a')
    mail.href = 'mailto:someone@example.com'
    mail.textContent = 'Write to us'
    mail.className = 'mail-probe'
    // Outside the app's root, which the page's script draws over.
    document.body.append(mail)
  })
  await page.locator('.clm-home .foot a', { hasText: 'Guide' }).click()
  await page.locator('a.mail-probe').click()
  await expect(page).toHaveURL(/\/$/)
  expect(await calls(page)).toEqual([])
})

test('the site sets no cookie and stores nothing but the light or dark choice', async ({ page, context }) => {
  // req:web.site/analytics-script@d3f71ebe
  await context.grantPermissions(['clipboard-read', 'clipboard-write'])
  await stub(page)
  await page.goto('/')
  await page.locator('.hero a.btn.p').click()
  await expect(page).toHaveURL(/\/guide\/quickstart$/)
  const block = page.locator('div[class*="language-"]').first()
  await block.hover()
  await block.locator('button.copy').click()
  expect((await calls(page)).length).toBe(2)
  const stored = await page.evaluate(() => ({
    cookie: document.cookie,
    local: Object.keys(localStorage).filter((k) => k !== 'vitepress-theme-appearance'),
    session: Object.keys(sessionStorage).filter((k) => k !== 'vitepress-theme-appearance'),
  }))
  expect(stored).toEqual({ cookie: '', local: [], session: [] })
  expect(await context.cookies()).toEqual([])
})

/** The first line of the first fenced code block of the repository's quickstart, trimmed. */
function quickstartFirstCommand(): string {
  const lines = readFileSync(join(repoDocs, 'guide/quickstart.md'), 'utf8').split('\n')
  const fence = lines.findIndex((l) => /^\s*(```|~~~)/.test(l))
  return lines[fence + 1].trim()
}

test("a code block's copy button reports the command", async ({ page, context }) => {
  // req:web.site/copy-command-event@1fe75b72
  await context.grantPermissions(['clipboard-read', 'clipboard-write'])
  await stub(page)
  await page.goto('/guide/quickstart')
  const block = page.locator('div[class*="language-"]').first()
  await block.hover()
  await block.locator('button.copy').click()
  expect(await calls(page)).toEqual([
    ['copy-command', { path: '/guide/quickstart', command: quickstartFirstCommand() }],
  ])
})

test('a selection copied from the homepage commands reports the command', async ({ page, context }) => {
  // req:web.site/copy-command-event@1fe75b72
  await context.grantPermissions(['clipboard-read', 'clipboard-write'])
  await stub(page)
  await page.goto('/')
  await page.evaluate(() => {
    const code = document.querySelector('.cmd code')!
    const range = document.createRange()
    range.selectNodeContents(code)
    const selection = window.getSelection()!
    selection.removeAllRanges()
    selection.addRange(range)
  })
  await page.evaluate(() => document.execCommand('copy'))
  expect(await calls(page)).toEqual([
    ['copy-command', { path: '/', command: '/plugin marketplace add ' + 'clunny' + '/clm' }],
  ])
})

test('a selection outside any command, an empty one, or one of a line break alone reports nothing', async ({
  page,
}) => {
  // req:web.site/copy-command-event@1fe75b72
  await stub(page)
  await page.goto('/')
  await page.evaluate(() => {
    const copy = (range: Range) => {
      const selection = window.getSelection()!
      selection.removeAllRanges()
      selection.addRange(range)
      document.execCommand('copy')
    }
    // The first screen's paragraph.
    const paragraph = document.createRange()
    paragraph.selectNodeContents(document.querySelector('.hero p')!)
    copy(paragraph)
    // An empty selection inside the first install command.
    const empty = document.createRange()
    const first = document.querySelector('.cmd code')!.firstChild!
    empty.setStart(first, 3)
    empty.setEnd(first, 3)
    copy(empty)
    // Only the line break between the closing block's first two commands.
    const block = document.querySelector('.final pre')!.firstChild!
    const at = block.textContent!.indexOf('\n')
    const lineBreak = document.createRange()
    lineBreak.setStart(block, at)
    lineBreak.setEnd(block, at + 1)
    copy(lineBreak)
  })
  expect(await calls(page)).toEqual([])
})
