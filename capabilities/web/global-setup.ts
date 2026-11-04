import { execFileSync } from 'node:child_process'
import { readFileSync } from 'node:fs'
import { dirname, join } from 'node:path'
import { fileURLToPath } from 'node:url'

// Builds what the tests run, as `web/` is deployed: the browser app into `web/dist/`, the
// Worker as a dev build, and the `clm` binary the scenarios use. The tests then reach the app
// only through that build.
export default function setup() {
  const root = join(dirname(fileURLToPath(import.meta.url)), '..', '..')
  const run = (cwd: string, command: string, ...args: string[]) => execFileSync(command, args, { cwd, stdio: 'inherit' })

  run(join(root, 'web'), 'npm', 'run', 'build')

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
}
