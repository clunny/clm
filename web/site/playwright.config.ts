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
    // No website ID, so the build holds no analytics script and the tests stub the service; a
    // star count given, so no build waits on GitHub (`github-link`).
    env: { SITE_OUT: '.test-dist/main', UMAMI_WEBSITE_ID: '', GITHUB_STARS: '1234', INSTALL_REPO: '' },
    url: `http://localhost:${port}/`,
    timeout: 180_000,
    reuseExistingServer: false,
  },
})
