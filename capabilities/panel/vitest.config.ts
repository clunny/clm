import { fileURLToPath } from 'node:url'
import { defineConfig } from 'vitest/config'

const here = (p: string) => fileURLToPath(new URL(p, import.meta.url))

export default defineConfig({
  // The panel's modules live outside this package: they import `claude-code`, which is the
  // fake engine, and draw with the engine's classic JSX on `h`.
  resolve: { alias: { 'claude-code': here('./fake-engine.ts') } },
  oxc: { jsx: { runtime: 'classic', pragma: 'h', pragmaFrag: 'Fragment' } },
  server: { fs: { strict: false } },
  test: {
    include: ['*.test.ts'],
    globalSetup: [here('./global-setup.ts')],
    setupFiles: [here('./setup.ts')],
    testTimeout: 60_000,
  },
})
