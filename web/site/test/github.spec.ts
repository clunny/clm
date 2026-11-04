import { test, expect } from '@playwright/test'
import { build, read } from './helpers'

const repo = 'clunny' + '/clm'

test('the first screen and the nav link to the repository with its star count', async ({ page }) => {
  // req:web.site/github-link@37bdaa86
  await page.setViewportSize({ width: 1280, height: 800 })
  await page.goto('/')
  const link = page.locator(`.hero a[href="https://github.com/${repo}"]`)
  await expect(link).toHaveText('1.2k')
  const box = await link.boundingBox()
  expect(box!.y + box!.height).toBeLessThanOrEqual(800)
  await page.goto('/guide/quickstart')
  await expect(page.locator(`.VPNavBar a[href="https://github.com/${repo}"]`).first()).toHaveText('1.2k')
})

test('a small count shows as it is, and a round one to a decimal', () => {
  test.setTimeout(180_000)
  // req:web.site/github-link@37bdaa86
  const small = build('stars-87', { GITHUB_STARS: '87' })
  expect(small.error).toBeUndefined()
  expect(read(small.out, 'index.html')).toContain('<span>87</span>')
  expect(read(small.out, 'guide/quickstart.html')).toContain('<span class="gh-count">87</span>')
  const round = build('stars-1950', { GITHUB_STARS: '1950' })
  expect(read(round.out, 'index.html')).toContain('<span>2.0k</span>')
})

test('an unreachable GitHub fails nothing and shows the link with no count', () => {
  test.setTimeout(180_000)
  // req:web.site/github-link@37bdaa86
  const { out, error } = build('stars-none')
  expect(error).toBeUndefined()
  const home = read(out, 'index.html')
  expect(home).toContain(`href="https://github.com/${repo}"`)
  expect(home).toContain('<span>GitHub</span>')
  expect(read(out, 'guide/quickstart.html')).toContain('<span class="gh-count">GitHub</span>')
})

test('the GitHub links name the install repository', () => {
  test.setTimeout(180_000)
  // req:web.site/github-link@37bdaa86
  const { out, error } = build('stars-other', { INSTALL_REPO: 'someone/clm-public', GITHUB_STARS: '5' })
  expect(error).toBeUndefined()
  const home = read(out, 'index.html')
  expect(home).toContain('href="https://github.com/someone/clm-public"')
  expect(home).not.toContain(`https://github.com/${repo}"`)
  expect(read(out, 'guide/quickstart.html')).toContain('href="https://github.com/someone/clm-public"')
})
