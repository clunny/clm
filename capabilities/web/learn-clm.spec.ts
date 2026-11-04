// `learn-clm`: a newcomer who may see the site reads the homepage's first screen on their
// phone, follows Get started and lands on the guide's quickstart with the install commands.
// Until launch the site is behind sign-in, so the newcomer signs in as an invited login first;
// the site and the app are the built Worker and assets, and only GitHub is the fake's.
import { expect, test } from '@playwright/test'

import { FakeGitHub } from './fake-github.ts'
import { invited, projectOn, repository, signIn, startApp } from './harness.ts'

test('a newcomer goes from the homepage to the quickstart', async ({ page }) => {
  // req:capability/learn-clm@053867b7
  const repo = repository()
  try {
    projectOn(repo)
    const fake = new FakeGitHub('capability', 'learn', repo.origin, invited()[0])
    const app = await startApp(fake)
    try {
      await signIn(page, app)
      await page.goto(`${app.base}/`)

      // The first screen says what CLM does: the headline, and the four stages of the loop.
      const headline = page.locator('.hero h1')
      await expect(headline).toBeVisible()
      expect((await headline.innerText()).replace(/\s+/g, ' ')).toBe(
        'Declare the intent. AI builds the code to match. CLM keeps it matching.',
      )
      await expect(page.locator('.hero-text p').first()).toContainText('CLM reconciles the code to it')
      // All of it is on the phone's first screen, not below the fold.
      await expect(headline).toBeInViewport()
      await expect(page.locator('.hero-text p').first()).toBeInViewport()
      await expect(page.locator('.hero a.btn.p')).toBeInViewport()

      // Get started leads to the guide's quickstart, with the commands that install CLM.
      const start = page.locator('.hero a.btn.p')
      await expect(start).toHaveText('Get started')
      await start.click()
      await page.waitForURL(/\/guide\/quickstart/)
      const body = page.locator('main')
      await expect(body).toContainText('/plugin marketplace add')
      await expect(body).toContainText('/plugin install clm@clm')
    } finally {
      await app.stop()
      // A request the fake did not answer is the cause of a failure above, so it is named first.
      expect(fake.unanswered).toEqual([])
    }
  } finally {
    repo.remove()
  }
})
