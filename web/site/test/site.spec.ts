// The site is built from this repository's guide, so an edit to it selects this file
// (`doc-readers-marked`):
// clm:reads docs/guide/**
// clm:reads docs/spec-format.md
import { test, expect } from '@playwright/test'
import { execFileSync, spawn } from 'node:child_process'
import { existsSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs'
import { createServer } from 'node:net'
import type { AddressInfo } from 'node:net'
import { tmpdir } from 'node:os'
import { join, resolve } from 'node:path'
import { unstable_readConfig } from 'wrangler'
import { build, filesUnder, read, siteDir } from './helpers'

test('the homepage first screen shows the headline, Get started and the four stages', async ({ page }) => {
  // req:web.site/homepage-first-screen@cfa9754e
  await page.setViewportSize({ width: 1280, height: 800 })
  await page.goto('/')
  const view = await page.evaluate(() => {
    const inView = (r: DOMRect) =>
      r.width > 0 && r.height > 0 && r.top >= 0 && r.bottom <= window.innerHeight
    // Text nodes, so an element running past the window can't hide its text; ancestors don't count.
    const texts: string[] = []
    const walker = document.createTreeWalker(document.body, NodeFilter.SHOW_TEXT)
    for (let n = walker.nextNode(); n; n = walker.nextNode()) {
      const range = document.createRange()
      range.selectNodeContents(n)
      const rects = Array.from(range.getClientRects())
      if (rects.some((r) => r.width > 0 && r.top < window.innerHeight && r.bottom > 0)) {
        texts.push(n.textContent ?? '')
      }
    }
    const stages = Array.from(document.querySelectorAll('.ring .st b'))
      .filter((e) => inView(e.getBoundingClientRect()))
      .map((e) => e.textContent)
    const cta = document.querySelector('.hero a.btn.p') as HTMLAnchorElement | null
    return {
      texts: texts.join('\n'),
      stages,
      cta: cta && inView(cta.getBoundingClientRect()) ? [cta.getAttribute('href'), cta.textContent] : null,
      headline: (document.querySelector('.hero h1') as HTMLElement).innerText,
    }
  })
  expect(view.headline.replace(/\s+/g, ' ')).toBe(
    'Declare the intent. AI builds the code to match. CLM keeps it matching.',
  )
  expect(view.stages).toEqual(['Declare', 'Build', 'Prove', 'Reconcile'])
  expect(view.cta).toEqual(['/guide/quickstart', 'Get started'])
  expect(view.texts).not.toContain('Kubernetes')
})

test('the homepage holds its five sections in order with the credits linked', async ({ page }) => {
  // req:web.site/homepage-sections@16b02280
  await page.goto('/')
  const headings = await page.locator('.clm-home .sec .k').allTextContents()
  expect(headings.map((h) => h.trim())).toEqual([
    'The split',
    'The loop',
    'Not a vibe-coded mess',
    'Solo or as a team',
    'Standing on shoulders',
  ])
  await expect(page.locator('.clm-home .sec').nth(1)).toContainText("CLM isn't a Kubernetes tool")
  const credits = page.locator('.clm-home .sec').nth(4)
  for (const href of [
    'https://arxiv.org/html/2606.27045v2',
    'https://kubernetes.io/docs/concepts/architecture/controller/',
    'https://github.com/obra/superpowers',
    'https://claude.com/blog/the-ai-native-sdlc-playbook',
  ]) {
    await expect(credits.locator(`a[href="${href}"]`)).toHaveCount(1)
  }
})

test('the install commands name the one configured repository', () => {
  test.setTimeout(180_000)
  // req:web.site/install-repository@2a80f121
  const { out, error } = build('repo', { INSTALL_REPO: 'someone/clm-public', GITHUB_STARS: '1' })
  expect(error).toBeUndefined()
  expect(read(out, 'index.html')).toContain('/plugin marketplace add someone/clm-public')
  expect(read(out, 'llms.txt')).toContain('/plugin marketplace add someone/clm-public')
  const app = /<script type="application\/ld\+json">([^<]*)<\/script>/.exec(read(out, 'index.html'))!
  expect(JSON.parse(app[1]).sameAs).toEqual(['https://github.com/someone/clm-public'])
  // No built file names the public repository but those holding text taken from the guide's
  // files or the posts, which are written text and may name it in their own install commands:
  // their pages and the scripts that carry them, their copies, the agent files and the feed.
  const writtenText = (f: string) =>
    /^(guide|blog)\//.test(f) || /^assets\/(guide|blog)_/.test(f) || f === 'llms-full.txt' || f === 'llms.txt'
  const needle = 'clunny' + '/clm'
  const named = filesUnder(out).filter((f) => !writtenText(f) && read(out, f).includes(needle))
  expect(named).toEqual([])
  // llms.txt names it only in the guide's own text: its summaries.
  const llms = read(out, 'llms.txt')
  expect(llms.slice(0, llms.indexOf('## Guide'))).not.toContain(needle)
  expect(llms.slice(llms.indexOf('## Optional'))).not.toContain(needle)
})

test('the install commands default to the public repository', () => {
  test.setTimeout(180_000)
  // req:web.site/install-repository@2a80f121
  for (const env of [{}, { INSTALL_REPO: '' }]) {
    const { out, error } = build('default-repo', { ...env, GITHUB_STARS: '1' })
    expect(error).toBeUndefined()
    expect(read(out, 'index.html'), JSON.stringify(env)).toContain('/plugin marketplace add ' + 'clunny' + '/clm')
  }
})

test('the homepage and the guide fit a phone without scrolling sideways', async ({ page }) => {
  // req:web.site/site-phone-layout@a07cbe01
  await page.setViewportSize({ width: 390, height: 800 })
  for (const path of ['/', '/guide/quickstart']) {
    await page.goto(path)
    const widths = await page.evaluate(() => ({
      doc: document.documentElement.scrollWidth,
      win: window.innerWidth,
    }))
    expect(widths.doc, path).toBeLessThanOrEqual(widths.win)
    // `clip` on the page hides overflow from `scrollWidth`: no element may run past the window.
    const wide = await page.evaluate(
      () =>
        Array.from(document.querySelectorAll('.clm-home *'))
          .filter((e) => e.getBoundingClientRect().right > window.innerWidth + 1)
          .map((e) => e.className),
    )
    expect(wide, path).toEqual([])
  }
  await page.goto('/')
  const h1 = await page.locator('.clm-home h1').boundingBox()
  const ring = await page.locator('.clm-home .ring').boundingBox()
  expect(ring!.y).toBeGreaterThanOrEqual(h1!.y + h1!.height)
})

test('npmrc ignores install scripts', () => {
  // req:web.site/site-no-install-scripts@3828bfd9
  // A user's own npm settings that turn install scripts on, and nothing in the environment.
  const home = mkdtempSync(join(tmpdir(), 'clm-site-npmrc-'))
  try {
    const userconfig = join(home, '.npmrc')
    writeFileSync(userconfig, 'ignore-scripts=false\n')
    const env: NodeJS.ProcessEnv = { ...process.env, npm_config_userconfig: userconfig }
    for (const key of Object.keys(env)) if (/^npm_config_ignore[-_]scripts$/i.test(key)) delete env[key]
    const got = execFileSync('npm', ['config', 'get', 'ignore-scripts'], { cwd: siteDir, env, encoding: 'utf8' })
    expect(got.trim()).toBe('true')
  } finally {
    rmSync(home, { recursive: true, force: true })
  }
})

/** A port nothing listens on right now, from the OS. */
async function freePort(): Promise<number> {
  const server = createServer()
  await new Promise<void>((ok) => server.listen(0, '127.0.0.1', ok))
  const { port } = server.address() as AddressInfo
  await new Promise((ok) => server.close(ok))
  return port
}

test("wrangler reads the site's own Worker, assets only, and serves the build with its 404 page", async () => {
  test.setTimeout(240_000)
  // req:web.site/site-worker@d088ecb7
  // The site built as `npm run build` builds it, with no `SITE_OUT`.
  const env: NodeJS.ProcessEnv = { ...process.env, GITHUB_STARS: '1', UMAMI_WEBSITE_ID: '' }
  delete env.SITE_OUT
  execFileSync('npm', ['run', 'build'], { cwd: siteDir, env, stdio: 'pipe' })
  const config = unstable_readConfig({ config: join(siteDir, 'wrangler.toml') }, {})
  expect(config.name).toBe('clm-site')
  expect(config.main).toBeUndefined()
  expect(config.routes).toEqual([{ pattern: 'clm-app.dev', custom_domain: true }])
  const assets = resolve(siteDir, config.assets!.directory!)
  expect(existsSync(join(assets, 'index.html'))).toBe(true)

  const port = await freePort()
  const dev = spawn('npx', ['wrangler', 'dev', '--ip', '127.0.0.1', '--port', String(port)], {
    cwd: siteDir,
    env: { ...process.env, WRANGLER_SEND_METRICS: 'false', CI: '1' },
    detached: true,
    stdio: 'ignore',
  })
  try {
    const base = `http://127.0.0.1:${port}`
    const deadline = Date.now() + 90_000
    let up = false
    while (!up && Date.now() < deadline) {
      up = await fetch(`${base}/`).then(() => true, () => false)
      if (!up) await new Promise((ok) => setTimeout(ok, 500))
    }
    expect(up, 'wrangler dev answered').toBe(true)
    expect((await fetch(`${base}/guide/quickstart`)).status).toBe(200)
    const missing = await fetch(`${base}/no-such-page`)
    expect(missing.status).toBe(404)
    expect(await missing.text()).toBe(readFileSync(join(assets, '404.html'), 'utf8'))
  } finally {
    try {
      process.kill(-dev.pid!, 'SIGTERM')
    } catch {
      // Already gone.
    }
  }
})

test('the nav reads How it works, Guide, Blog, Get started, Web app, and every link to the app reads Web app', async ({
  page,
}) => {
  // req:web.site/nav-web-app@d9d68241
  for (const path of ['/', '/guide/quickstart']) {
    await page.goto(path)
    const nav = await page.locator('.VPNavBarMenu a').allTextContents()
    expect(nav.map((t) => t.trim()).slice(0, 5), path).toEqual([
      'How it works',
      'Guide',
      'Blog',
      'Get started',
      'Web app',
    ])
    await expect(page.locator('.VPNavBarMenu a', { hasText: 'Blog' })).toHaveAttribute('href', '/blog/')
    await expect(page.locator('.VPNavBarMenu a', { hasText: 'Web app' })).toHaveAttribute('href', '/app/')
    const toApp = await page.locator('a[href="/app/"]').allTextContents()
    expect(toApp.length, path).toBeGreaterThan(0)
    for (const text of toApp) expect(text.trim(), path).toBe('Web app')
    expect(await page.getByRole('link', { name: 'Sign in' }).count(), path).toBe(0)
  }
})

test("every link to the app loads /app/ from the server, not a page of the site's own", async ({ page }) => {
  // req:web.site/nav-web-app@d9d68241
  for (const [path, link] of [
    ['/guide/quickstart', '.VPNavBarMenu a:has-text("Web app")'],
    ['/', '.clm-home .foot a:has-text("Web app")'],
  ]) {
    await page.goto(path)
    // Once the app is up: VitePress's router takes a click only after hydration.
    await page.waitForLoadState('networkidle')
    const asked = page.waitForRequest(
      (r) => r.resourceType() === 'document' && new URL(r.url()).pathname === '/app/',
      { timeout: 10_000 },
    )
    await page.locator(link).click()
    await asked
  }
})
