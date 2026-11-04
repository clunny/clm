// `review-capability-cost`: after rules serving two capabilities have been built and landed,
// the person opens the review page on their phone, sees each capability's cost, and records a
// decision. The history is made as a person's would be, with `clm effort record` reading
// fixture transcripts (Claude Code lies beyond the boundary) and `clm land`.
import { expect, test } from '@playwright/test'
import { mkdirSync, writeFileSync } from 'node:fs'
import { join } from 'node:path'

import { FakeGitHub } from './fake-github.ts'
import { ROOT_SPEC, clm, git, invited, repository, signIn, startApp, viewReady, write } from './harness.ts'

const capability = (slug: string) =>
  `### Capability: ${slug}\nA person can ${slug}.\n\n#### Why\nw\n\n#### Scenario: s\n- GIVEN g\n- WHEN w\n- THEN t\n`

const node = (id: string, serves: string, dir: string) =>
  `---\ntitle: ${id}\nstatus: active\nowners: []\npaths: ["${dir}/**"]\nserves: [capability/${serves}]\n---\n\n## Contract\n${id} does r.\n\n## Requirements\n\n### Requirement: r\n${id} SHALL r.\n\n#### Scenario: r\n- GIVEN g\n- WHEN w\n- THEN t\n`

/** The marker a test cites a requirement of `id` with: built so this file never holds it whole. */
const marker = (id: string, version: string) => `# ${'req' + ':'}${id}/r@${version}\ntrue\n`

test('a person sees what each capability cost and decides to keep one', async ({ page }) => {
  // req:capability/review-capability-cost@9872aed9
  const repo = repository()
  try {
    write(repo.work, 'specs/SPEC.md', ROOT_SPEC)
    write(repo.work, 'specs/a/SPEC.md', node('a', 'p', 'a'))
    write(repo.work, 'specs/b/SPEC.md', node('b', 'q', 'b'))
    write(repo.work, 'CAPABILITIES.md', `# Capabilities\n\n## Capabilities\n\n${capability('p')}\n${capability('q')}`)
    write(repo.work, 'clm.toml', '[land]\ncheck = "true"\n')
    write(repo.work, '.gitignore', '.clm/\n.claude/worktrees/\ntarget/\n')
    repo.commit('specs')
    git(repo.work, 'push', '-q', '-u', 'origin', 'main')

    // The transcripts of the sessions that built the two nodes, as Claude Code keeps them.
    const transcripts = join(repo.top, 'projects')
    const encoded = repo.work.replace(/[^A-Za-z0-9]/g, '-')
    mkdirSync(join(transcripts, encoded), { recursive: true })
    const entry = (branch: string, id: string, minute: number, tokens: [number, number]) =>
      JSON.stringify({
        sessionId: `session-${branch.slice(-1)}`,
        gitBranch: branch,
        cwd: repo.work,
        timestamp: `2026-10-30T10:${String(minute).padStart(2, '0')}:00.000Z`,
        type: 'assistant',
        message: { id, model: 'claude-opus-5-5', usage: { input_tokens: tokens[0], output_tokens: tokens[1] } },
      })
    writeFileSync(
      join(transcripts, encoded, 'sessions.jsonl'),
      [
        entry('impl/a', 'ma1', 0, [40_000, 8_000]),
        entry('impl/a', 'ma2', 5, [30_000, 6_000]),
        entry('impl/b', 'mb1', 10, [10_000, 2_000]),
        entry('impl/b', 'mb2', 12, [8_000, 1_000]),
      ].join('\n') + '\n',
    )

    // Each node is built on its own branch, its effort recorded, and landed.
    for (const id of ['a', 'b']) {
      git(repo.work, 'switch', '-q', '-c', `impl/${id}`, 'origin/main')
      const version = clm(repo.work, ['hashes', id])
        .split('\n')
        .find((l) => l.startsWith('requirement:r '))!
        .split(' ')[1]
        .slice(0, 8)
      write(repo.work, `${id}/t.sh`, marker(id, version))
      repo.commit(`build ${id}`)
      git(repo.work, 'push', '-q', '-u', 'origin', `impl/${id}`)
      clm(repo.work, ['effort', 'record', '--branch', `impl/${id}`, '--transcripts', transcripts], { HOME: repo.top })
      expect(clm(repo.work, ['land', `impl/${id}`], { HOME: repo.top })).toMatch(/^landed /m)
      git(repo.work, 'fetch', '-q', 'origin')
    }

    const reviewer = invited()[0]
    const fake = new FakeGitHub('capability', 'cost', repo.origin, reviewer)
    const app = await startApp(fake)
    try {
      await signIn(page, app)
      await viewReady(page, app, 'capabilities')
      await page.goto(`${app.base}/app/#/capability/cost/capabilities`)
      // Each capability's cost, on a phone's cards.
      const p = page.locator('a.ccard[data-slug=p]')
      const q = page.locator('a.ccard[data-slug=q]')
      await expect(p).toContainText('84k tok')
      await expect(q).toContainText('21k tok')
      await expect(p).toContainText('never decided')

      await p.click()
      await expect(page.locator('section[data-sec=cost]')).toContainText('tokens')
      await page.locator('button.value', { hasText: 'high' }).click()
      await page.locator('button.decide', { hasText: 'Keep' }).click()
      await expect(page.locator('section[data-sec=decisions]')).toContainText(`keep (value high) by ${reviewer}/web`)
    } finally {
      await app.stop()
      // A request the fake did not answer is the cause of a failure above, so it is named first.
      expect(fake.unanswered).toEqual([])
    }

    // The decision is on the person's web ledger branch in the repository.
    const records = git(repo.origin, 'show', `clm/ledger/${reviewer}/web:records.jsonl`)
    expect(records).toMatch(/"decided".*"keep"|"keep".*"decided"/s)
    expect(records).toContain('capability/p')
  } finally {
    repo.remove()
  }
})

const entry = (slug: string, rule: string) =>
  `### Constraint: ${slug}\n${slug} SHALL hold.\n\n#### Scenario: s\n- GIVEN g\n- WHEN w\n- THEN t\n\n#### Enforced by\n- ${rule}\n`

test('a person sees which architectural constraint is held and decides to keep one', async ({ page }) => {
  // req:capability/review-capability-cost@9872aed9
  const repo = repository()
  try {
    write(repo.work, 'specs/SPEC.md', ROOT_SPEC)
    write(repo.work, 'specs/a/SPEC.md', node('a', 'p', 'a'))
    write(repo.work, 'specs/b/SPEC.md', node('b', 'p', 'b'))
    write(repo.work, 'CAPABILITIES.md', `# Capabilities\n\n## Capabilities\n\n${capability('p')}`)
    write(repo.work, 'ARCHITECTURE.md', `# Architecture\n\n## Constraints\n\n${entry('held-one', 'a/r')}\n${entry('unheld-one', 'b/r')}`)
    write(repo.work, 'clm.toml', '[land]\ncheck = "true"\n')
    write(repo.work, '.gitignore', '.clm/\n.claude/worktrees/\ntarget/\n')
    repo.commit('specs')
    git(repo.work, 'push', '-q', '-u', 'origin', 'main')

    // Only node `a` is built: its rule is current, node `b`'s is still pending.
    const transcripts = join(repo.top, 'projects')
    const encoded = repo.work.replace(/[^A-Za-z0-9]/g, '-')
    mkdirSync(join(transcripts, encoded), { recursive: true })
    writeFileSync(
      join(transcripts, encoded, 'sessions.jsonl'),
      JSON.stringify({
        sessionId: 'session-a',
        gitBranch: 'impl/a',
        cwd: repo.work,
        timestamp: '2026-10-30T10:00:00.000Z',
        type: 'assistant',
        message: { id: 'ma1', model: 'claude-opus-5-5', usage: { input_tokens: 40_000, output_tokens: 8_000 } },
      }) + '\n',
    )
    git(repo.work, 'switch', '-q', '-c', 'impl/a', 'origin/main')
    const version = clm(repo.work, ['hashes', 'a'])
      .split('\n')
      .find((l) => l.startsWith('requirement:r '))!
      .split(' ')[1]
      .slice(0, 8)
    write(repo.work, 'a/t.sh', marker('a', version))
    repo.commit('build a')
    git(repo.work, 'push', '-q', '-u', 'origin', 'impl/a')
    clm(repo.work, ['effort', 'record', '--branch', 'impl/a', '--transcripts', transcripts], { HOME: repo.top })
    expect(clm(repo.work, ['land', 'impl/a'], { HOME: repo.top })).toMatch(/^landed /m)
    git(repo.work, 'fetch', '-q', 'origin')

    const reviewer = invited()[0]
    const fake = new FakeGitHub('capability', 'arch', repo.origin, reviewer)
    const app = await startApp(fake)
    try {
      // GitHub tells the Worker of the landed push, and the repo's object finishes the marker
      // index that says which rules are built.
      await app.push(git(repo.origin, 'rev-parse', 'main').trim())
      await signIn(page, app)
      await viewReady(page, app, 'architecture', (body) => body.indexing === false)
      await page.goto(`${app.base}/app/#/capability/arch/architecture`)
      // Each requirement's proof and cost, on a phone's cards.
      const held = page.locator('a.ccard[data-slug=held-one]')
      const unheld = page.locator('a.ccard[data-slug=unheld-one]')
      await expect(held).toContainText('held 1/1')
      await expect(held).toContainText('48k tok')
      await expect(unheld).toContainText('held 0/1')
      await expect(unheld).toContainText('0 tok')

      await unheld.click()
      await expect(page.locator('li.rule[data-rule="b/r"]')).toHaveAttribute('data-state', 'pending')
      await page.locator('button.value', { hasText: 'high' }).click()
      await page.locator('button.decide', { hasText: 'Keep' }).click()
      await expect(page.locator('section[data-sec=decisions]')).toContainText(`keep (value high) by ${reviewer}/web`)
    } finally {
      await app.stop()
      expect(fake.unanswered).toEqual([])
    }

    const records = git(repo.origin, 'show', `clm/ledger/${reviewer}/web:records.jsonl`)
    expect(records).toMatch(/architecture\/unheld-one.*keep|keep.*architecture\/unheld-one/s)
  } finally {
    repo.remove()
  }
})
