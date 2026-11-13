// `watch-machine-load`: a person watches the machine's CPU load, available memory and free disk
// beside their Claude Code chat. The boundary is the panel's plugin directory, loaded as Claude
// Code loads it (`fake-engine.ts`); the pane runs the real `clm` and `git`.
import { execFileSync } from 'node:child_process'
import { cpSync, mkdirSync, mkdtempSync, realpathSync, rmSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { dirname, join } from 'node:path'
import { fileURLToPath } from 'node:url'
import { afterAll, expect, test } from 'vitest'

import { createEngine, type Engine } from './fake-engine.ts'

const repository = join(dirname(fileURLToPath(import.meta.url)), '..', '..')

const git = (cwd: string, ...args: string[]) =>
  execFileSync('git', args, {
    cwd,
    encoding: 'utf8',
    env: { ...process.env, GIT_CONFIG_GLOBAL: '/dev/null', GIT_CONFIG_NOSYSTEM: '1' },
  })

const made: string[] = []
const engines: Engine[] = []
// The pane's `clm` writes under the project, so each engine ends its processes first.
afterAll(async () => {
  await Promise.all(engines.map(e => e.close()))
  made.forEach(d => rmSync(d, { recursive: true, force: true }))
})

test('the pane charts cpu, mem and disk after 30 seconds', async () => {
  // req:capability/watch-machine-load@0e544722
  const top = realpathSync(mkdtempSync(join(tmpdir(), 'clm-panel-load-')))
  made.push(top)
  const person = join(top, 'person')
  git(top, 'init', '-q', '-b', 'main', 'person')
  git(person, 'config', 'user.name', 'Ann')
  git(person, 'config', 'user.email', 'ann@example.com')
  mkdirSync(join(person, 'specs'), { recursive: true })
  writeFileSync(join(person, 'specs/SPEC.md'), '---\ntitle: R\nstatus: active\nowners: []\n---\n\n## Contract\nThe product.\n')
  writeFileSync(join(person, 'clm.toml'), 'host = "none"\n')
  git(person, 'add', '-A')
  git(person, 'commit', '-q', '-m', 'specs')

  const pluginRoot = join(top, 'plugins', 'clm-panel')
  cpSync(join(repository, 'plugin-panel'), pluginRoot, {
    recursive: true,
    filter: from => !from.includes('node_modules'),
  })
  const engine = await createEngine({ pluginRoot, cwd: person, path: process.env.CLM_CAPABILITY_BIN_DIR! })
  engines.push(engine)
  const opened = await engine.command('clm:view')
  expect(opened.text).toContain('Opened')

  await engine.advance(30_000)
  const rows = () => engine.pane().split('\n').map(l => l.trim())
  // The first run of a debug `clm` can take a while.
  await expect.poll(() => rows().some(l => l.startsWith('disk')), { timeout: 20_000 }).toBe(true)
  for (const name of ['cpu', 'mem', 'disk']) {
    const row = rows().find(l => l.startsWith(name))
    expect(row, `a ${name} row in:\n${engine.pane()}`).toBeDefined()
    // The row charts the readings taken so far: block characters after the name.
    expect(row).toMatch(/[▁-█]/)
  }
})
