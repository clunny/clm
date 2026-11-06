import { test, expect } from '@playwright/test'
import { execFileSync } from 'node:child_process'
import { readFileSync, readdirSync, statSync } from 'node:fs'
import { join, relative, resolve } from 'node:path'

const siteDir = resolve(import.meta.dirname, '..')

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
  // req:web.site/install-repository@b53df95f
  const out = join(siteDir, '.test-dist', 'repo')
  execFileSync('npx', ['vitepress', 'build', '.'], {
    cwd: siteDir,
    env: { ...process.env, SITE_OUT: out, INSTALL_REPO: 'someone/clm-public', GITHUB_STARS: '1' },
    stdio: 'pipe',
  })
  const html = readFileSync(join(out, 'index.html'), 'utf8')
  expect(html).toContain('/plugin marketplace add someone/clm-public')
  // The default repository is written in the config alone.
  const needle = 'clunny' + '/clm'
  const named: string[] = []
  const walk = (dir: string) => {
    for (const name of readdirSync(dir)) {
      if (['node_modules', 'dist', '.test-dist', 'test-results', 'cache', 'package-lock.json'].includes(name)) continue
      const path = join(dir, name)
      if (statSync(path).isDirectory()) walk(path)
      else if (readFileSync(path, 'utf8').includes(needle)) named.push(relative(siteDir, path))
    }
  }
  walk(siteDir)
  expect(named).toEqual(['.vitepress/config.ts'])
})

test('the install commands default to the public repository', () => {
  test.setTimeout(180_000)
  // req:web.site/install-repository@b53df95f
  const out = join(siteDir, '.test-dist', 'default-repo')
  const env = { ...process.env, SITE_OUT: out, GITHUB_STARS: '1' }
  delete env.INSTALL_REPO
  execFileSync('npx', ['vitepress', 'build', '.'], { cwd: siteDir, env, stdio: 'pipe' })
  const html = readFileSync(join(out, 'index.html'), 'utf8')
  expect(html).toContain('/plugin marketplace add ' + 'clunny' + '/clm')
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
  // req:web.site/site-no-install-scripts@5ee0d14e
  const npmrc = readFileSync(join(siteDir, '.npmrc'), 'utf8')
  expect(npmrc.split('\n').map((l) => l.trim())).toContain('ignore-scripts=true')
})

test("wrangler.toml declares the site's own Worker, assets only", () => {
  // req:web.site/site-worker@0fb56a72
  const toml = readFileSync(join(siteDir, 'wrangler.toml'), 'utf8')
    .split('\n')
    .map((l) => l.trim())
    .filter((l) => l !== '' && !l.startsWith('#'))
  expect(toml).toContain('name = "clm-site"')
  expect(toml.filter((l) => /^main\s*=/.test(l))).toEqual([])
  expect(toml.filter((l) => /^routes\s*=/.test(l))).toEqual([
    'routes = [{ pattern = "clm-app.dev", custom_domain = true }]',
  ])
  const assets = toml.slice(toml.indexOf('[assets]') + 1)
  expect(assets).toContain('not_found_handling = "404-page"')
  // Where `npm run build` writes the site by default.
  expect(assets).toContain('directory = "./dist"')
  expect(readFileSync(join(siteDir, '.vitepress/config.ts'), 'utf8')).toContain("process.env.SITE_OUT || 'dist'")
})

test('the nav reads How it works, Guide, Blog, Get started, Web app, and every link to the app reads Web app', async ({
  page,
}) => {
  // req:web.site/nav-web-app@c1d913a8
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
