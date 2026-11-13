// `watch-beside-chat`: a person watches the project's state update in a pane beside their
// Claude Code chat. The boundary is the panel's plugin directory, loaded as Claude Code loads
// it (`fake-engine.ts`); the pane runs the real `clm` and `git`.
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

const clm = (cwd: string, ...args: string[]) =>
  execFileSync(join(process.env.CLM_CAPABILITY_BIN_DIR!, 'clm'), args, {
    cwd,
    encoding: 'utf8',
    env: { ...process.env, GIT_CONFIG_GLOBAL: '/dev/null', GIT_CONFIG_NOSYSTEM: '1', XDG_CONFIG_HOME: '/nonexistent' },
  })

const write = (dir: string, path: string, content: string) => {
  mkdirSync(dirname(join(dir, path)), { recursive: true })
  writeFileSync(join(dir, path), content)
}

const NODE_A = `---
title: A
status: active
owners: []
paths: ["src/**", "tests/**"]
---

## Contract
A does x.

## Requirements

### Requirement: x
A SHALL x.

#### Scenario: x
- GIVEN g
- WHEN w
- THEN t
`

const made: string[] = []
const engines: Engine[] = []
// The pane's `clm` writes under the project, so each engine ends its processes first.
afterAll(async () => {
  await Promise.all(engines.map(e => e.close()))
  made.forEach(d => rmSync(d, { recursive: true, force: true }))
})

const project = () => {
  const top = realpathSync(mkdtempSync(join(tmpdir(), 'clm-panel-')))
  made.push(top)
  const origin = join(top, 'origin.git')
  const person = join(top, 'person')
  git(top, 'init', '-q', '--bare', '-b', 'main', 'origin.git')
  git(top, 'init', '-q', '-b', 'main', 'person')
  git(person, 'config', 'user.name', 'Ann')
  git(person, 'config', 'user.email', 'ann@example.com')
  git(person, 'remote', 'add', 'origin', origin)
  write(person, 'specs/SPEC.md', '---\ntitle: R\nstatus: active\nowners: []\n---\n\n## Contract\nThe product.\n')
  write(person, 'specs/a/SPEC.md', NODE_A)
  write(person, 'clm.toml', 'host = "git"\n\n[land]\ncheck = "true"\n')
  write(person, '.gitignore', '.clm/\ntarget/\n')
  git(person, 'add', '-A')
  git(person, 'commit', '-q', '-m', 'specs')
  git(person, 'push', '-q', '-u', 'origin', 'main')
  return { top, origin, person }
}

// The plugin is installed alone: its root holds no `clm` shim, so the pane runs the `clm` on `PATH`.
const openPane = async (top: string, cwd: string) => {
  const pluginRoot = join(top, 'plugins', 'clm-panel')
  cpSync(join(repository, 'plugin-panel'), pluginRoot, {
    recursive: true,
    filter: from => !from.includes('node_modules'),
  })
  const engine = await createEngine({ pluginRoot, cwd, path: process.env.CLM_CAPABILITY_BIN_DIR! })
  engines.push(engine)
  return engine
}

test('the pane shows the new state when a build lands, without being asked', async () => {
  // req:capability/watch-beside-chat@109098d4
  const { top, origin, person } = project()

  // The pane, open beside the chat. The plugin is installed alone: its root holds no `clm`
  // shim, so the pane runs the `clm` on `PATH`.
  const engine = await openPane(top, person)
  const opened = await engine.command('clm:view')
  expect(opened.text).toContain('Opened')
  await expect.poll(() => engine.pane()).toContain('Flight')
  await engine.press('Flight')
  await expect.poll(() => engine.pane()).toContain('1 components pending (1 ready)')
  expect(engine.pane()).toMatch(/\ba +P5 +ready/)

  // A build lands, from another clone, as a build session would.
  const builder = join(top, 'builder')
  git(top, 'clone', '-q', origin, 'builder')
  git(builder, 'config', 'user.name', 'Bot')
  git(builder, 'config', 'user.email', 'bot@example.com')
  git(builder, 'switch', '-q', '-c', 'impl/a')
  const version = clm(builder, 'hashes', 'a')
    .split('\n')
    .find(l => l.startsWith('requirement:x '))!
    .split(' ')[1]!
    .slice(0, 8)
  write(builder, 'tests/t_x.sh', `# ${'req' + ':'}a/x@${version}\ntrue\n`)
  git(builder, 'add', '-A')
  git(builder, 'commit', '-q', '-m', 'build a')
  git(builder, 'push', '-q', '-u', 'origin', 'impl/a')
  expect(clm(builder, 'land', 'impl/a')).toMatch(/^landed /m)

  // The test's clock passes the pane's fetch and redraw intervals; nothing is asked of it.
  await engine.advance(150_000)
  await expect.poll(() => engine.pane(), { timeout: 20_000 }).toContain('0 components pending')
  expect(engine.pane()).not.toMatch(/\ba +P5 +ready/)
})

test('Drain on the Queue tab stops the loop launching anything new, and the tab says so', async () => {
  // req:capability/watch-beside-chat@109098d4
  const { top, person } = project()
  const engine = await openPane(top, person)

  // The build loop runs: it holds the clone's lease and has a build of `a` running.
  expect(clm(person, 'build', 'start', '--session-model', 'opus')).toMatch(/^launch build:a /m)
  clm(person, 'build', 'launched', 'build:a', 'task-1')
  // A second component becomes ready on `main` meanwhile: a build the loop would launch next.
  write(person, 'specs/b/SPEC.md', NODE_A.replace('title: A', 'title: B'))
  git(person, 'add', '-A')
  git(person, 'commit', '-q', '-m', 'b')
  git(person, 'push', '-q', 'origin', 'main')
  await engine.command('clm:view')
  await engine.press('Queue')
  const slow = { timeout: 20_000 }
  await expect.poll(() => engine.pane(), slow).toContain('[Drain]')
  expect(engine.pane()).not.toContain('stop requested')

  await engine.press('Drain')
  await expect.poll(() => engine.pane(), slow).toContain('stop requested: drain')
  await expect.poll(() => engine.pane(), slow).toContain('Drain · requested')

  // The loop obeys it: its next step launches nothing, though `b` is ready.
  expect(clm(person, 'view', 'flight')).toMatch(/\bb +P5 +ready/)
  expect(clm(person, 'build', 'step')).not.toMatch(/^launch /m)
})
