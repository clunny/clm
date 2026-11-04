import { test, expect, type Page } from '@playwright/test'
import { build, filesUnder, mainDist, read } from './helpers'

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
  // req:web.site/analytics-script@8ab7cdcc
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
  // req:web.site/analytics-script@8ab7cdcc
  const named = filesUnder(mainDist).filter((f) => read(mainDist, f).includes(service))
  expect(named).toEqual([])
})

test('Get started reports a click and still goes on', async ({ page }) => {
  // req:web.site/click-events@dbda97a8
  await stub(page)
  await page.goto('/')
  await page.locator('.hero a.btn.p').click()
  await expect(page).toHaveURL(/\/guide\/quickstart$/)
  // The in-site navigation keeps the page, and the stub with it.
  expect(await calls(page)).toEqual([['click', { label: 'Get started', path: '/' }]])
})

test('How it works and a nav link report clicks with their own text', async ({ page }) => {
  // req:web.site/click-events@dbda97a8
  await stub(page)
  await page.goto('/')
  await page.locator('.hero a.btn.s').click()
  await page.locator('.VPNavBarMenu a', { hasText: 'Guide' }).click()
  expect(await calls(page)).toEqual([
    ['click', { label: 'How it works', path: '/' }],
    ['click', { label: 'Guide', path: '/' }],
  ])
})

test('a link to another host reports an outbound click only', async ({ page }) => {
  // req:web.site/click-events@dbda97a8
  await stub(page)
  await page.goto('/')
  // Navigation held: the click is reported, and the page stays to be read.
  await page.evaluate(() => document.addEventListener('click', (e) => e.preventDefault()))
  await page.locator('a[href="https://github.com/obra/superpowers"]').click()
  expect(await calls(page)).toEqual([
    ['outbound', { url: 'https://github.com/obra/superpowers', path: '/' }],
  ])
})

test('clicks work with no analytics service and raise no error', async ({ page }) => {
  // req:web.site/click-events@dbda97a8
  const errors: string[] = []
  page.on('pageerror', (e) => errors.push(e.message))
  page.on('console', (m) => m.type() === 'error' && errors.push(m.text()))
  await page.goto('/')
  await page.locator('.hero a.btn.p').click()
  await expect(page).toHaveURL(/\/guide\/quickstart$/)
  expect(errors).toEqual([])
})

test("a code block's copy button reports the command", async ({ page, context }) => {
  // req:web.site/copy-command-event@cea0c255
  await context.grantPermissions(['clipboard-read', 'clipboard-write'])
  await stub(page)
  await page.goto('/guide/quickstart')
  const block = page.locator('div[class*="language-"]').first()
  await block.hover()
  await block.locator('button.copy').click()
  expect(await calls(page)).toEqual([
    ['copy-command', { path: '/guide/quickstart', command: '/plugin marketplace add ' + 'clunny' + '/clm' }],
  ])
})

test('a selection copied from the homepage commands reports the command', async ({ page, context }) => {
  // req:web.site/copy-command-event@cea0c255
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

test('a selection outside any command reports nothing', async ({ page }) => {
  // req:web.site/copy-command-event@cea0c255
  await stub(page)
  await page.goto('/')
  await page.evaluate(() => {
    const range = document.createRange()
    range.selectNodeContents(document.querySelector('.hero p')!)
    const selection = window.getSelection()!
    selection.removeAllRanges()
    selection.addRange(range)
    document.execCommand('copy')
  })
  expect(await calls(page)).toEqual([])
})
