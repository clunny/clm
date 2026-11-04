import { defineConfig, devices } from '@playwright/test'

// Each spec starts its own Miniflare around the built Worker and app, on a phone-sized
// Chromium page. The builds run once in `global-setup.ts`.
export default defineConfig({
  testDir: '.',
  testMatch: '*.spec.ts',
  globalSetup: './global-setup.ts',
  // The suites of checkouts land in parallel on one machine.
  timeout: 120_000,
  expect: { timeout: 30_000 },
  workers: 1,
  reporter: process.env.CI ? 'github' : 'list',
  use: { ...devices['Pixel 7'] },
})
