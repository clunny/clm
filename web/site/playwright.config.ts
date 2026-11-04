import { defineConfig } from '@playwright/test'
import { previewPort } from './test/preview-port'

// The tests drive the built site: the server builds it into `.test-dist/main` and previews it,
// on a port of this run's own (`test/preview-port.ts`), so land checks of checkouts running at
// the same time never serve on each other's port.
const port = previewPort()

export default defineConfig({
  testDir: 'test',
  fullyParallel: false,
  workers: 1,
  use: { baseURL: `http://localhost:${port}` },
  webServer: {
    command: `npm run build && npx vitepress preview . --port ${port}`,
    env: { SITE_OUT: '.test-dist/main' },
    url: `http://localhost:${port}/`,
    timeout: 180_000,
    reuseExistingServer: false,
  },
})
