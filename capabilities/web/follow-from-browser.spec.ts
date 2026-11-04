// `follow-from-browser`: an invited reviewer opens a change's page on their phone and
// approves it, and `clm land` counts the approval. The app is the built Worker and assets;
// only GitHub is the fake's.
import { expect, test } from '@playwright/test'

import { FakeGitHub } from './fake-github.ts'
import { CAPABILITIES, clm, git, invited, projectOn, repository, signIn, startApp, viewReady, write } from './harness.ts'

test('a reviewer approves a change on a phone and clm land counts it', async ({ page }) => {
  // req:capability/follow-from-browser@bf6823a4
  const repo = repository()
  try {
    // An unsigned repository, and a spec branch that needs review: it edits CAPABILITIES.md.
    projectOn(repo)
    git(repo.work, 'switch', '-q', '-c', 'spec/cap')
    write(repo.work, 'CAPABILITIES.md', CAPABILITIES.replace('A person can c.', 'A person can c, quickly.'))
    repo.commit('c, quickly')
    git(repo.work, 'push', '-q', '-u', 'origin', 'spec/cap')

    // Before anyone approves, `clm land` refuses it.
    const clone = repo.clone('lander')
    expect(() => clm(clone, ['land', 'spec/cap'])).toThrow(/gate\/approval/)

    const reviewer = invited()[0]
    const fake = new FakeGitHub('capability', 'follow', repo.origin, reviewer)
    const app = await startApp(fake)
    try {
      await signIn(page, app)
      await viewReady(page, app, 'snapshot')
      await page.goto(`${app.base}/app/#/capability/follow/prs/spec%2Fcap`)
      const approve = page.locator('button.approve')
      await expect(approve).toBeVisible()
      await approve.click()
      await expect(page.locator('.review')).toContainText('Approved.')
      await expect(approve).toBeDisabled()
    } finally {
      await app.stop()
      // A request the fake did not answer is the cause of a failure above, so it is named first.
      expect(fake.unanswered).toEqual([])
    }

    // The approval is on the reviewer's web ledger branch in the repository.
    const records = git(repo.origin, 'show', `clm/ledger/${reviewer}/web:records.jsonl`)
    expect(records).toContain('"approved"')
    expect(records).toContain('spec/cap')

    // `clm land`, on a clone of that repository, counts it.
    const landed = clm(clone, ['land', 'spec/cap'])
    expect(landed).toMatch(/^landed /m)
  } finally {
    repo.remove()
  }
})
