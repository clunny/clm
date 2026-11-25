// The site is built from this repository's guide, so an edit to it selects this file
// (`doc-readers-marked`):
// clm:reads docs/guide/**
// clm:reads docs/spec-format.md
import { test, expect } from '@playwright/test'
import { createServer, type ServerResponse } from 'node:http'
import type { AddressInfo } from 'node:net'
import { build, buildAsync, read } from './helpers'

const repo = 'clunny' + '/clm'

test('the first screen and the nav link to the repository with its star count', async ({ page }) => {
  // req:web.site/github-link@8a304cc1
  await page.setViewportSize({ width: 1280, height: 800 })
  await page.goto('/')
  const link = page.locator(`.hero a[href="https://github.com/${repo}"]`)
  await expect(link).toHaveText('1.2k')
  const box = await link.boundingBox()
  expect(box!.y + box!.height).toBeLessThanOrEqual(800)
  await page.goto('/guide/quickstart')
  await expect(page.locator(`.VPNavBar a[href="https://github.com/${repo}"]`).first()).toHaveText('1.2k')
})

type Box = { x: number; y: number; width: number; height: number }
const centre = (b: Box) => b.y + b.height / 2

test('the mark sits before the count on one line, centred with its neighbours', async ({ page }) => {
  // req:web.site/github-link@8a304cc1
  await page.setViewportSize({ width: 1280, height: 800 })
  for (const path of ['/', '/guide/quickstart']) {
    await page.goto(path)
    const nav = page.locator(`.VPNavBar a[href="https://github.com/${repo}"]`).first()
    const links = [{ link: nav, label: nav.locator('.gh-count') }]
    if (path === '/') {
      const button = page.locator(`.hero a[href="https://github.com/${repo}"]`)
      links.push({ link: button, label: button.locator('span') })
    }
    for (const { link, label } of links) {
      const mark = (await link.locator('svg').boundingBox())!
      const count = (await label.boundingBox())!
      expect(mark.x + mark.width, path).toBeLessThanOrEqual(count.x)
      expect(Math.abs(centre(mark) - centre(count)), path).toBeLessThanOrEqual(2)
    }
    const guide = (await page.locator('.VPNavBarMenu a', { hasText: 'Guide' }).boundingBox())!
    const navCount = (await links[0].label.boundingBox())!
    expect(Math.abs(centre(navCount) - centre(guide)), path).toBeLessThanOrEqual(2)
  }
  await page.goto('/')
  const button = (await page.locator(`.hero a[href="https://github.com/${repo}"]`).boundingBox())!
  const started = (await page.locator('.hero a.btn.p').boundingBox())!
  expect(Math.abs(centre(button) - centre(started))).toBeLessThanOrEqual(2)
})

test('a small count shows as it is, and a round one to a decimal', () => {
  test.setTimeout(180_000)
  // req:web.site/github-link@8a304cc1
  const small = build('stars-87', { GITHUB_STARS: '87' })
  expect(small.error).toBeUndefined()
  expect(read(small.out, 'index.html')).toContain('<span>87</span>')
  expect(read(small.out, 'guide/quickstart.html')).toContain('<span class="gh-count">87</span>')
  const round = build('stars-1950', { GITHUB_STARS: '1950' })
  expect(read(round.out, 'index.html')).toContain('<span>2.0k</span>')
})

test('an unreachable GitHub fails nothing and shows the link with no count', () => {
  test.setTimeout(180_000)
  // req:web.site/github-link@8a304cc1
  const { out, output, error } = build('stars-none')
  expect(error).toBeUndefined()
  expect(output).toMatch(/warning: the star count .* couldn't be read/)
  expectNoCount(out)
})

/** The built links name the repository and read `GitHub`, with no count. */
function expectNoCount(out: string, label = '') {
  const home = read(out, 'index.html')
  expect(home, label).toContain(`href="https://github.com/${repo}"`)
  expect(home, label).toContain('<span>GitHub</span>')
  expect(read(out, 'guide/quickstart.html'), label).toContain('<span class="gh-count">GitHub</span>')
}

/** A stand-in for GitHub's API on a port of the system's choosing: `answer` handles each request,
 * which is recorded with its method, path and `Authorization` header. */
async function fakeApi(answer: (res: ServerResponse) => void) {
  const asked: { method?: string; url?: string; auth?: string }[] = []
  const server = createServer((req, res) => {
    asked.push({ method: req.method, url: req.url, auth: req.headers.authorization })
    answer(res)
  })
  await new Promise<void>((ok) => server.listen(0, '127.0.0.1', ok))
  const { port } = server.address() as AddressInfo
  return {
    url: `http://127.0.0.1:${port}`,
    asked,
    close: () => {
      server.closeAllConnections()
      return new Promise<void>((ok) => server.close(() => ok()))
    },
  }
}

const json = (status: number, body: string) => (res: ServerResponse) => {
  res.statusCode = status
  res.setHeader('content-type', 'application/json')
  res.end(body)
}

test("the count is asked of GitHub's API with the token", async () => {
  test.setTimeout(180_000)
  // req:web.site/github-link@8a304cc1
  const api = await fakeApi(json(200, '{"stargazers_count": 87}'))
  try {
    const { out, error } = await buildAsync('stars-api', { GITHUB_API: api.url, GITHUB_TOKEN: 't' })
    expect(error).toBeUndefined()
    expect(api.asked).toContainEqual({ method: 'GET', url: `/repos/${repo}`, auth: 'Bearer t' })
    expect(read(out, 'index.html')).toContain('<span>87</span>')
    expect(read(out, 'guide/quickstart.html')).toContain('<span class="gh-count">87</span>')
  } finally {
    await api.close()
  }
})

test("a count that can't be used shows no count and warns", async () => {
  test.setTimeout(300_000)
  // req:web.site/github-link@8a304cc1
  for (const given of ['abc', '-5', '12.5', '1e3']) {
    const { out, output, error } = build('stars-bad', { GITHUB_STARS: given })
    expect(error, given).toBeUndefined()
    expect(output, given).toContain('warning:')
    expectNoCount(out, given)
  }
  const answers: [string, (res: ServerResponse) => void][] = [
    ['500', json(500, '{"message": "down"}')],
    ['{}', json(200, '{}')],
    ['-5', json(200, '{"stargazers_count": -5}')],
    ['never', () => {}],
  ]
  for (const [label, answer] of answers) {
    const api = await fakeApi(answer)
    try {
      const { out, output, error } = await buildAsync('stars-bad', { GITHUB_API: api.url })
      expect(error, label).toBeUndefined()
      expect(api.asked.length, label).toBeGreaterThan(0)
      expect(output, label).toMatch(/warning: the star count .* couldn't be read/)
      expectNoCount(out, label)
    } finally {
      await api.close()
    }
  }
})

test('the GitHub links name the install repository', () => {
  test.setTimeout(180_000)
  // req:web.site/github-link@8a304cc1
  const { out, error } = build('stars-other', { INSTALL_REPO: 'someone/clm-public', GITHUB_STARS: '5' })
  expect(error).toBeUndefined()
  const home = read(out, 'index.html')
  expect(home).toContain('href="https://github.com/someone/clm-public"')
  expect(home).not.toContain(`https://github.com/${repo}"`)
  expect(read(out, 'guide/quickstart.html')).toContain('href="https://github.com/someone/clm-public"')
})
