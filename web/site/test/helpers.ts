import { execFileSync } from 'node:child_process'
import { mkdirSync, readFileSync, readdirSync, rmSync, statSync, writeFileSync } from 'node:fs'
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
  for (const key of ['UMAMI_WEBSITE_ID', 'GITHUB_STARS', 'INSTALL_REPO', 'GUIDE_DIR']) delete base[key]
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

/** The built file at `rel` under `out`. */
export const read = (out: string, rel: string) => readFileSync(join(out, rel), 'utf8')

/** Every file under `dir`, as paths relative to it. */
export function filesUnder(dir: string, rel = ''): string[] {
  return readdirSync(join(dir, rel)).flatMap((name) => {
    const path = join(rel, name)
    return statSync(join(dir, path)).isDirectory() ? filesUnder(dir, path) : [path]
  })
}
