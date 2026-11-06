import { execFileSync } from 'node:child_process'
import { mkdtempSync, readFileSync, rmSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { dirname, join } from 'node:path'
import { fileURLToPath } from 'node:url'

// Builds what the tests run, as the two Workers are deployed: the browser app into `web/dist/`,
// the public site (`web/site/`, a package of its own) into a directory of this run's, the
// Worker as a dev build, and the `clm` binary the scenarios use. The tests then reach the app
// only through those builds.
export default function setup(): () => void {
  const root = join(dirname(fileURLToPath(import.meta.url)), '..', '..')
  const run = (cwd: string, command: string, ...args: string[]) => execFileSync(command, args, { cwd, stdio: 'inherit' })

  run(join(root, 'web'), 'npm', 'run', 'build')

  // The site is no part of `web`'s build (`web/app-at-app-path`): build it here, and the harness
  // serves it at `/` beside the app, as the zone does.
  const site = mkdtempSync(join(tmpdir(), 'clm-web-capability-site-'))
  run(join(root, 'web', 'site'), 'npm', 'ci', '--ignore-scripts')
  execFileSync('npm', ['run', 'build'], { cwd: join(root, 'web', 'site'), stdio: 'inherit', env: { ...process.env, SITE_OUT: site } })
  process.env.CLM_CAPABILITY_SITE_DIR = site

  // The version of `worker-build` that `web/package.json` pins for the release build.
  const pinned = JSON.parse(readFileSync(join(root, 'web', 'package.json'), 'utf8')).scripts['build:worker'] as string
  const version = /--version (\S+)/.exec(pinned)?.[1]
  if (!version) throw new Error("web/package.json's build:worker pins no worker-build version")
  run(join(root, 'crates', 'clm-web'), 'cargo', 'install', '-q', 'worker-build', '--version', version, '--locked')
  run(join(root, 'crates', 'clm-web'), 'worker-build', '--dev')

  run(root, 'cargo', 'build', '-q', '-p', 'clm')
  const metadata = execFileSync('cargo', ['metadata', '--format-version', '1', '--no-deps'], { cwd: root, maxBuffer: 1 << 28 })
  const target = JSON.parse(metadata.toString()).target_directory as string
  process.env.CLM_CAPABILITY_BIN_DIR = join(target, 'debug')
  return () => rmSync(site, { recursive: true, force: true })
}
