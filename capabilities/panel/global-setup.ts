import { execFileSync } from 'node:child_process'
import { dirname, join } from 'node:path'
import { fileURLToPath } from 'node:url'

// Builds the `clm` binary from the repository and tells the tests where it is, as the
// directory to put first on `PATH`.
export default function setup() {
  const root = join(dirname(fileURLToPath(import.meta.url)), '..', '..')
  execFileSync('cargo', ['build', '-q', '-p', 'clm'], { cwd: root, stdio: 'inherit' })
  const metadata = execFileSync('cargo', ['metadata', '--format-version', '1', '--no-deps'], {
    cwd: root,
    maxBuffer: 1 << 28,
  })
  const target = JSON.parse(metadata.toString()).target_directory as string
  process.env.CLM_CAPABILITY_BIN_DIR = join(target, 'debug')
}
