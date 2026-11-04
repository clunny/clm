import { defineConfig, devices } from '@playwright/test';
import { previewPort } from './test/preview-port';

// The built app under `vite preview`, with the Worker's API mocked in each test. The preview
// listens on a port of this run's own (`test/preview-port.ts`), so the suites of checkouts
// checked in parallel never serve on, or tear down, each other's port.
const port = previewPort();
const url = `http://localhost:${port}`;

export default defineConfig({
  testDir: 'e2e',
  fullyParallel: true,
  // Checkouts land in parallel on one machine, so this suite runs while other builds compile
  // and run their own suites. A first paint that takes a second idle takes over five under
  // that load, which failed assertions that were only slow (`test/playwright-config.test.ts`).
  // An assertion that holds costs nothing more: it passes as soon as it does.
  timeout: 60_000,
  expect: { timeout: 20_000 },
  reporter: process.env.CI ? 'github' : 'list',
  use: { baseURL: url, serviceWorkers: 'allow' },
  projects: [{ name: 'chromium', use: { ...devices['Desktop Chrome'] } }],
  webServer: {
    command: `npm run build && npm run preview -- --port ${port} --strictPort`,
    url,
    reuseExistingServer: !process.env.CI,
    timeout: 120_000,
  },
});
