// The shared harness of the web capability specs: a person's repository with a bare
// `origin` that the fake GitHub serves, the app as deployed (the built Worker and assets,
// run by Miniflare), and the sign-in a reviewer does.
import { execFileSync, spawnSync } from 'node:child_process'
import { createHmac, generateKeyPairSync, randomBytes } from 'node:crypto'
import { mkdirSync, mkdtempSync, readFileSync, realpathSync, rmSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { dirname, join } from 'node:path'
import { fileURLToPath } from 'node:url'
import { expect, type Page } from '@playwright/test'
import { Miniflare, Response as MiniflareResponse, convertV4MiniflareOptions } from 'miniflare'

import { FakeGitHub } from './fake-github.ts'

const WEBHOOK_SECRET = 'webhook-secret'
const root = join(dirname(fileURLToPath(import.meta.url)), '..', '..')
const GIT_ENV = { ...process.env, GIT_CONFIG_GLOBAL: '/dev/null', GIT_CONFIG_NOSYSTEM: '1', XDG_CONFIG_HOME: '/nonexistent' }
for (const v of Object.keys(GIT_ENV).filter((k) => k.startsWith('CLM_') || ['GIT_DIR', 'GIT_WORK_TREE', 'GIT_INDEX_FILE'].includes(k))) delete GIT_ENV[v as keyof typeof GIT_ENV]

export const git = (cwd: string, ...args: string[]): string => execFileSync('git', args, { cwd, encoding: 'utf8', env: GIT_ENV })

/** Runs the `clm` binary the global setup built; a failure is the error's `stdout` and `stderr`. */
export function clm(cwd: string, args: string[], env: Record<string, string> = {}): string {
  const result = spawnSync(join(process.env.CLM_CAPABILITY_BIN_DIR!, 'clm'), args, {
    cwd,
    encoding: 'utf8',
    env: { ...GIT_ENV, CLM_CAPABILITY_BIN_DIR: process.env.CLM_CAPABILITY_BIN_DIR, ...env },
  })
  const printed = `${result.stdout}${result.stderr}`
  if (result.status !== 0) throw new Error(`clm ${args.join(' ')} failed: ${printed}`)
  return printed
}

export const write = (dir: string, path: string, content: string) => {
  mkdirSync(dirname(join(dir, path)), { recursive: true })
  writeFileSync(join(dir, path), content)
}

/** The logins `web/dist/_clm/invited.json` lists, which the build wrote. */
export const invited = (): string[] => JSON.parse(readFileSync(join(root, 'web', 'dist', '_clm', 'invited.json'), 'utf8'))

/** A repository on a person's machine, with a bare `origin` the fake GitHub serves. */
export interface Repository {
  top: string
  work: string
  origin: string
  /** Commits everything in the work tree. */
  commit(message: string): void
  /** Another person's clone of `origin`, with its own identity. */
  clone(name: string): string
  remove(): void
}

export function repository(): Repository {
  const top = realpathSync(mkdtempSync(join(tmpdir(), 'clm-web-capability-')))
  const origin = join(top, 'origin.git')
  const work = join(top, 'work')
  git(top, 'init', '-q', '--bare', '-b', 'main', 'origin.git')
  git(top, 'init', '-q', '-b', 'main', 'work')
  git(work, 'config', 'user.name', 'Ann')
  git(work, 'config', 'user.email', 'ann@example.com')
  git(work, 'remote', 'add', 'origin', origin)
  return {
    top,
    work,
    origin,
    commit(message) {
      git(work, 'add', '-A')
      git(work, 'commit', '-q', '-m', message)
    },
    clone(name) {
      git(top, 'clone', '-q', origin, name)
      git(join(top, name), 'config', 'user.name', name)
      git(join(top, name), 'config', 'user.email', `${name}@example.com`)
      return join(top, name)
    },
    remove: () => rmSync(top, { recursive: true, force: true }),
  }
}

export const NODE_A = `---
title: A
status: active
owners: []
paths: ["src/**", "tests/**"]
serves: [capability/c]
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

export const ROOT_SPEC = '---\ntitle: R\nstatus: active\nowners: []\n---\n\n## Contract\nThe product.\n'

/** A running app: the built Worker and assets, with the fake GitHub behind it. */
export interface App {
  base: string
  fake: FakeGitHub
  /** GitHub's signed `push` event for `main` of the fake's repository, now at `sha`. */
  push(sha: string): Promise<void>
  stop(): Promise<void>
}

/**
 * The app as deployed (`web/wrangler.toml`): the Worker built with `worker-build`, the assets
 * with their routes, KV, and the `Live` and `Replica` Durable Objects on SQLite. Miniflare doesn't read the
 * config, so its bindings are repeated here; one the copy lacks shows as a failing test.
 */
export async function startApp(fake: FakeGitHub): Promise<App> {
  const build = join(root, 'crates', 'clm-web', 'build')
  const key = generateKeyPairSync('rsa', { modulusLength: 2048 }).privateKey.export({ type: 'pkcs8', format: 'pem' }) as string
  const mf = new Miniflare(convertV4MiniflareOptions({
    host: '127.0.0.1',
    port: 0,
    modules: [
      { type: 'ESModule', path: join(build, 'index.js') },
      { type: 'CompiledWasm', path: join(build, 'index_bg.wasm') },
    ],
    modulesRoot: build,
    compatibilityDate: '2026-10-01',
    kvNamespaces: ['CLM'],
    durableObjects: {
      LIVE: { className: 'Live', useSQLite: true },
      REPLICA: { className: 'Replica', useSQLite: true },
    },
    assets: {
      directory: join(root, 'web', 'dist'),
      binding: 'ASSETS',
      routerConfig: { has_user_worker: true },
      run_worker_first: ['/*', '!/app/*', '!/sw.js'],
      assetConfig: { not_found_handling: 'none' },
    },
    bindings: {
      GITHUB_APP_ID: '12345',
      GITHUB_CLIENT_ID: 'Iv1.capability',
      GITHUB_CLIENT_SECRET: 'client-secret',
      GITHUB_WEBHOOK_SECRET: WEBHOOK_SECRET,
      COOKIE_KEY: randomBytes(32).toString('base64'),
      GITHUB_APP_PRIVATE_KEY: key,
    },
    outboundService: async (request) => {
      const res = await fake.handle(request as unknown as Request)
      return new MiniflareResponse(await res.text(), { status: res.status, headers: [...res.headers] })
    },
  }))
  const url = await mf.ready
  return {
    base: url.origin,
    fake,
    async push(sha) {
      const body = JSON.stringify({
        ref: 'refs/heads/main',
        after: sha,
        repository: { full_name: `${fake.owner}/${fake.repo}`, default_branch: 'main' },
        installation: { id: 1 },
      })
      const signature = `sha256=${createHmac('sha256', WEBHOOK_SECRET).update(body).digest('hex')}`
      const res = await fetch(`${url.origin}/webhook`, {
        method: 'POST',
        headers: { 'x-github-event': 'push', 'x-hub-signature-256': signature, 'x-github-delivery': randomBytes(8).toString('hex') },
        body,
      })
      if (res.status !== 200) throw new Error(`the push webhook answered ${res.status}`)
    },
    stop: () => mf.dispose(),
  }
}

/**
 * Signs in as the app's first invited login by the Worker's own flow. The Worker's login
 * route sends the browser to GitHub's authorize page with a `state`, kept in a cookie too; the
 * person's approval there is the redirect back to the Worker's callback with that `state` and
 * a code, which the fake exchanges.
 */
export async function signIn(page: Page, app: App): Promise<void> {
  const login = await page.context().request.get(`${app.base}/auth/login`, { maxRedirects: 0 })
  const authorize = new URL(login.headers()['location'])
  if (authorize.origin !== 'https://github.com' || authorize.pathname !== '/login/oauth/authorize') {
    throw new Error(`/auth/login sent the browser to ${authorize}`)
  }
  const state = authorize.searchParams.get('state')!
  await page.goto(`${app.base}/auth/callback?code=fake-code&state=${encodeURIComponent(state)}`)
  // Signed in: the session is set wherever the callback sent the browser. Open the app, so the
  // harness holds before and after the callback points at `/app/`.
  await page.waitForURL((url) => url.origin === app.base && !url.pathname.startsWith('/auth/'))
  await page.goto(`${app.base}/app/`)
  // Wait until the app has sent itself on to the first repo: a test's own navigation made
  // before then is overwritten by that redirect.
  await page.waitForURL((url) => url.pathname === '/app/' && url.hash !== '')
}

/**
 * Waits until the repository's `Replica` object has the `view` the app reads (`ready` judges
 * it), as the app does after its first request is answered 503 `{"preparing"}`: the object
 * builds the repo from the fake GitHub on its alarm. Signed in with `page`.
 */
export async function viewReady(
  page: Page,
  app: App,
  view: string,
  ready: (body: any) => boolean = () => true,
): Promise<void> {
  const url = `${app.base}/api/repos/${app.fake.owner}/${app.fake.repo}/${view}`
  await expect
    .poll(
      async () => {
        // From inside the page, where the session cookie is.
        const res = await page.evaluate(async (u) => {
          const r = await fetch(u)
          return { status: r.status, text: await r.text() }
        }, url)
        if (res.status !== 200) return `${res.status} ${res.text}`
        return ready(JSON.parse(res.text)) || 'not ready'
      },
      { message: `the ${view} view is ready`, timeout: 60_000, intervals: [250, 500, 1000] },
    )
    .toBe(true)
}

export const CAPABILITIES =
  '# Capabilities\n\n## Capabilities\n\n### Capability: c\nA person can c.\n\n#### Why\nw\n\n#### Scenario: s\n- GIVEN g\n- WHEN w\n- THEN t\n'

/** A CLM repository on `main`, pushed to `origin`, that lands on the checks `true`. */
export function projectOn(repo: Repository): void {
  write(repo.work, 'specs/SPEC.md', ROOT_SPEC)
  write(repo.work, 'specs/a/SPEC.md', NODE_A)
  write(repo.work, 'CAPABILITIES.md', CAPABILITIES)
  write(repo.work, 'clm.toml', '[land]\ncheck = "true"\n')
  write(repo.work, '.gitignore', '.clm/\n.claude/worktrees/\ntarget/\n')
  repo.commit('specs')
  git(repo.work, 'push', '-q', '-u', 'origin', 'main')
}
