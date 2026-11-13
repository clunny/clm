// `follow-from-browser`: an invited reviewer opens a change's page on their phone and
// approves it, and `clm land` counts the approval; an invited person taps an answer to a
// question on the home page, and `clm triage` prints it. The app is the built Worker and assets;
// only GitHub is the fake's.
import { expect, test } from '@playwright/test'

import { FakeGitHub } from './fake-github.ts'
import { CAPABILITIES, clm, git, invited, projectOn, repository, signIn, startApp, viewReady, write } from './harness.ts'

test('a reviewer approves a change on a phone and clm land counts it', async ({ page }) => {
  // req:capability/follow-from-browser@bca3b7e6
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

test('a person answers a question on a phone and clm triage prints the answer', async ({ page }) => {
  // req:capability/follow-from-browser@bca3b7e6
  const repo = repository()
  try {
    // A `question` item that lists the choices `yes` and `no`.
    projectOn(repo)
    write(
      repo.work,
      'backlog/ship-it.md',
      '---\ntitle: "Ship it?"\nkind: question\ncomponents: [a]\nchoices: [yes, no]\n---\nShall we ship?\n',
    )
    repo.commit('a question')
    git(repo.work, 'push', '-q', 'origin', 'main')

    const person = invited()[0]
    const fake = new FakeGitHub('capability', 'answer', repo.origin, person)
    const app = await startApp(fake)
    try {
      await signIn(page, app)
      await viewReady(page, app, 'snapshot')
      await page.goto(`${app.base}/app/#/capability/answer/`)
      const yes = page.locator('button[data-answer=yes]')
      await expect(yes).toBeVisible()
      await yes.click()
      await expect(yes).toHaveAttribute('aria-pressed', 'true')
    } finally {
      await app.stop()
      expect(fake.unanswered).toEqual([])
    }

    // `clm triage`, on a clone of the repository, prints the answer under the item's line.
    const clone = repo.clone('triager')
    const lines = clm(clone, ['triage']).split('\n')
    const at = lines.findIndex((l) => l.startsWith('human ship-it '))
    expect(at).toBeGreaterThanOrEqual(0)
    expect(lines[at + 1]).toMatch(new RegExp(`^answered ship-it ${person}/web \\S+ yes$`))
  } finally {
    repo.remove()
  }
})
