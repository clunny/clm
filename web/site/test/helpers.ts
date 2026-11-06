import { execFileSync } from 'node:child_process'
import { createServer } from 'node:http'
import type { AddressInfo } from 'node:net'
import { existsSync, mkdirSync, readFileSync, readdirSync, rmSync, statSync, writeFileSync } from 'node:fs'
import { join, resolve } from 'node:path'

export const siteDir = resolve(import.meta.dirname, '..')
export const repoDocs = resolve(siteDir, '../../docs')
/** The build the browser tests serve (`playwright.config.ts`). */
export const mainDist = join(siteDir, '.test-dist', 'main')

/** Builds the site into `.test-dist/<name>` with `env` added to the environment; returns the
 * output, or the failure's text. `UMAMI_WEBSITE_ID` and `GITHUB_STARS` start unset, and the
 * GitHub API address refuses connections, so no build waits on the network. */
export function build(
  name: string,
  env: Record<string, string> = {},
): { out: string; error?: string } {
  const out = join(siteDir, '.test-dist', name)
  rmSync(out, { recursive: true, force: true })
  const base: NodeJS.ProcessEnv = { ...process.env, SITE_OUT: out, GITHUB_API: 'http://127.0.0.1:9' }
  for (const key of ['UMAMI_WEBSITE_ID', 'GITHUB_STARS', 'INSTALL_REPO', 'GUIDE_DIR', 'BLOG_DIR']) delete base[key]
  try {
    execFileSync('npx', ['vitepress', 'build', '.'], {
      cwd: siteDir,
      env: { ...base, ...env },
      stdio: 'pipe',
      maxBuffer: 64 * 1024 * 1024,
    })
    return { out }
  } catch (e) {
    const err = e as { stdout?: Buffer; stderr?: Buffer; message: string }
    return { out, error: `${err.stdout ?? ''}${err.stderr ?? ''}${err.message}` }
  }
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
