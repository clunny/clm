import { spawn, spawnSync } from 'node:child_process'
import { createServer } from 'node:http'
import type { AddressInfo } from 'node:net'
import { existsSync, mkdirSync, readFileSync, readdirSync, rmSync, statSync, writeFileSync } from 'node:fs'
import { join, resolve } from 'node:path'

export const siteDir = resolve(import.meta.dirname, '..')
export const repoDocs = resolve(siteDir, '../../docs')
/** The build the browser tests serve (`playwright.config.ts`). */
export const mainDist = join(siteDir, '.test-dist', 'main')

/** What a build left: where it wrote, all it printed, and, when it failed, the failure's text. */
export type Built = { out: string; output: string; error?: string }

/** The environment a build runs in: `env` added to this process's, with `UMAMI_WEBSITE_ID`,
 * `GITHUB_STARS`, `GITHUB_TOKEN`, `INSTALL_REPO`, `GUIDE_DIR` and `BLOG_DIR` unset first and the
 * GitHub API address one that refuses connections, so no build waits on the network. */
function buildEnv(out: string, env: Record<string, string>): NodeJS.ProcessEnv {
  const base: NodeJS.ProcessEnv = { ...process.env, SITE_OUT: out, GITHUB_API: 'http://127.0.0.1:9' }
  for (const key of ['UMAMI_WEBSITE_ID', 'GITHUB_STARS', 'GITHUB_TOKEN', 'INSTALL_REPO', 'GUIDE_DIR', 'BLOG_DIR']) {
    delete base[key]
  }
  return { ...base, ...env }
}

const built = (out: string, status: number | null, output: string, failure?: string): Built =>
  status === 0 ? { out, output } : { out, output, error: `${output}${failure ?? `exit ${status}`}` }

/** Builds the site into `.test-dist/<name>` with `env` added to the environment (`buildEnv`). */
export function build(name: string, env: Record<string, string> = {}): Built {
  const out = join(siteDir, '.test-dist', name)
  rmSync(out, { recursive: true, force: true })
  const run = spawnSync('npx', ['vitepress', 'build', '.'], {
    cwd: siteDir,
    env: buildEnv(out, env),
    encoding: 'utf8',
    maxBuffer: 64 * 1024 * 1024,
  })
  return built(out, run.status, `${run.stdout ?? ''}${run.stderr ?? ''}`, run.error?.message)
}

/** `build`, leaving this process free to answer requests while it runs, as a test's own
 * stand-in for GitHub's API must. */
export function buildAsync(name: string, env: Record<string, string> = {}): Promise<Built> {
  const out = join(siteDir, '.test-dist', name)
  rmSync(out, { recursive: true, force: true })
  return new Promise((done) => {
    const child = spawn('npx', ['vitepress', 'build', '.'], { cwd: siteDir, env: buildEnv(out, env) })
    let output = ''
    child.stdout.on('data', (d) => (output += d))
    child.stderr.on('data', (d) => (output += d))
    child.on('error', (e) => done(built(out, null, output, e.message)))
    child.on('close', (status) => done(built(out, status, output)))
  })
}

/** A fixture guide under `.test-dist/<name>-src/docs/guide`, with its sibling `spec-format.md`. */
export function fixture(name: string, files: Record<string, string>): string {
  const docs = join(siteDir, '.test-dist', `${name}-src`, 'docs')
  rmSync(docs, { recursive: true, force: true })
  mkdirSync(join(docs, 'guide'), { recursive: true })
  writeFileSync(join(docs, 'spec-format.md'), '# Spec format\n\nThe format.\n')
  for (const [file, text] of Object.entries(files)) writeFileSync(join(docs, 'guide', file), text)
  return join(docs, 'guide')
}

/** A fixture blog under `.test-dist/<name>-blog`: each key a file name, each value its text. */
export function blogFixture(name: string, files: Record<string, string>): string {
  const dir = join(siteDir, '.test-dist', `${name}-blog`)
  rmSync(dir, { recursive: true, force: true })
  mkdirSync(dir, { recursive: true })
  for (const [file, text] of Object.entries(files)) writeFileSync(join(dir, file), text)
  return dir
}

/** A post's file: the frontmatter from `front`, then `body`. */
export function post(front: Record<string, string | number | boolean | string[]>, body: string): string {
  const lines = Object.entries(front).map(([k, v]) => `${k}: ${JSON.stringify(v)}`)
  return `---\n${lines.join('\n')}\n---\n\n${body}\n`
}

/** `n` whitespace-separated words. */
export const words = (n: number) => Array.from({ length: n }, () => 'word').join(' ')

const types: Record<string, string> = { html: 'text/html', css: 'text/css', js: 'text/javascript' }

/** Serves a built site from `dir` as Cloudflare's assets do: `/a/b` is `a/b.html`, a missing
 * path is `404.html` with status 404. */
export async function serve(dir: string): Promise<{ url: string; close: () => Promise<void> }> {
  const server = createServer((req, res) => {
    const path = decodeURIComponent((req.url ?? '/').split(/[?#]/)[0])
    const candidates = [path, `${path}.html`, join(path, 'index.html')].map((p) => join(dir, p))
    const file = candidates.find((f) => existsSync(f) && statSync(f).isFile())
    res.statusCode = file ? 200 : 404
    const target = file ?? join(dir, '404.html')
    res.setHeader('content-type', types[target.split('.').pop() ?? ''] ?? 'application/octet-stream')
    res.end(readFileSync(target))
  })
  await new Promise<void>((ok) => server.listen(0, '127.0.0.1', ok))
  const { port } = server.address() as AddressInfo
  return { url: `http://127.0.0.1:${port}`, close: () => new Promise((ok) => server.close(() => ok())) }
}

/** The built file at `rel` under `out`. */
export const read = (out: string, rel: string) => readFileSync(join(out, rel), 'utf8')

/** Every file under `dir`, as paths relative to it. */
export function filesUnder(dir: string, rel = ''): string[] {
  return readdirSync(join(dir, rel)).flatMap((name) => {
    const path = join(rel, name)
    return statSync(join(dir, path)).isDirectory() ? filesUnder(dir, path) : [path]
  })
}
