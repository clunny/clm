// `see-site-use`: the maintainer sees a visitor's path from the homepage to a copied install
// command. The site is built a second time with a test website ID, in a directory of its own, and
// served as static files, as the deployment's assets are. The analytics service lies beyond the
// boundary: its script is a fake that records the tag that loaded it and every `track` call.
import { execFileSync } from 'node:child_process'
import { existsSync, mkdtempSync, readFileSync, rmSync, statSync } from 'node:fs'
import { createServer, type Server } from 'node:http'
import { tmpdir } from 'node:os'
import { dirname, extname, join } from 'node:path'
import { fileURLToPath } from 'node:url'
import { expect, test, type Page } from '@playwright/test'

const root = join(dirname(fileURLToPath(import.meta.url)), '..', '..')
const WEBSITE_ID = '11111111-2222-3333-4444-555555555555'
const SCRIPT_URL = 'https://cloud.umami.is/script.js'

// Records, in the page's session storage so that it outlives a full page load, each load of the
// script (the website ID and Do Not Track attributes of its tag, and the page) and each call.
const FAKE_SCRIPT = `
(() => {
  const log = (entry) => {
    const all = JSON.parse(sessionStorage.getItem('umami-log') || '[]')
    all.push(entry)
    sessionStorage.setItem('umami-log', JSON.stringify(all))
  }
  const tag = document.currentScript
  log({ loaded: location.pathname, id: tag.getAttribute('data-website-id'), dnt: tag.getAttribute('data-do-not-track') })
  window.umami = { track: (name, data) => log({ track: name, data }) }
})()
`

const TYPES: Record<string, string> = {
  '.html': 'text/html',
  '.js': 'text/javascript',
  '.css': 'text/css',
  '.json': 'application/json',
  '.svg': 'image/svg+xml',
  '.png': 'image/png',
  '.txt': 'text/plain',
  '.woff2': 'font/woff2',
}

/** Serves `dir` as static files, with `/x` answered by `x.html` as the deployment does. */
function serve(dir: string): Promise<{ base: string; server: Server }> {
  const server = createServer((req, res) => {
    let path: string
    try {
      path = decodeURIComponent(new URL(req.url!, 'http://x').pathname)
    } catch {
      res.writeHead(400).end('bad request')
      return
    }
    const candidates = [path, `${path}.html`, join(path, 'index.html')]
    const file = candidates.map((c) => join(dir, c)).find((f) => existsSync(f) && statSync(f).isFile())
    if (!file) {
      res.writeHead(404).end('not found')
      return
    }
    res.writeHead(200, { 'content-type': TYPES[extname(file)] ?? 'application/octet-stream' }).end(readFileSync(file))
  })
  return new Promise((resolve) => {
    server.listen(0, '127.0.0.1', () => {
      const address = server.address() as { port: number }
      resolve({ base: `http://127.0.0.1:${address.port}`, server })
    })
  })
}

const log = (page: Page) =>
  page.evaluate(() => JSON.parse(sessionStorage.getItem('umami-log') || '[]') as Record<string, unknown>[])

test('a visitor goes from the homepage to a copied install command', async ({ page, context }) => {
  // req:capability/see-site-use@afd02cfc
  const out = mkdtempSync(join(tmpdir(), 'clm-site-use-'))
  // The code host's API address refuses connections, so the build waits on no network.
  const env = { ...process.env, SITE_OUT: out, UMAMI_WEBSITE_ID: WEBSITE_ID, GITHUB_STARS: '1234', GITHUB_API: 'http://127.0.0.1:9' }
  delete env.INSTALL_REPO
  delete env.GUIDE_DIR
  const outside: string[] = []
  let server: Server | undefined
  try {
    execFileSync('npx', ['vitepress', 'build', '.'], { cwd: join(root, 'web', 'site'), env, stdio: 'pipe', maxBuffer: 1 << 28 })
    const served = await serve(out)
    server = served.server
    const base = served.base
    await context.grantPermissions(['clipboard-read', 'clipboard-write'])
    await page.route('**/*', async (route) => {
      const url = route.request().url()
      if (url === SCRIPT_URL) return route.fulfill({ contentType: 'text/javascript', body: FAKE_SCRIPT })
      if (!url.startsWith(base) && !url.startsWith('data:')) {
        outside.push(url)
        return route.abort()
      }
      return route.continue()
    })

    await page.goto(`${base}/`)
    await page.locator('.hero a.btn.p').click()
    await page.waitForURL(/\/guide\/quickstart/)
    // A visitor who arrives on the quickstart afresh loads the service there too.
    await page.reload()
    const block = page.locator('div[class*="language-"]').first()
    const command = ((await block.locator('pre').innerText()).split('\n')[0] ?? '').trim()
    expect(command).toContain('/plugin marketplace add')
    await block.hover()
    await block.locator('button.copy').click()

    const entries = await log(page)
    expect(entries.filter((e) => 'loaded' in e)).toEqual([
      { loaded: '/', id: WEBSITE_ID, dnt: 'true' },
      { loaded: '/guide/quickstart', id: WEBSITE_ID, dnt: 'true' },
    ])
    expect(entries.filter((e) => 'track' in e)).toEqual([
      { track: 'click', data: { label: 'Get started', path: '/' } },
      { track: 'copy-command', data: { path: '/guide/quickstart', command } },
    ])
    expect(outside).toEqual([])
  } finally {
    server?.close()
    rmSync(out, { recursive: true, force: true })
  }
})
