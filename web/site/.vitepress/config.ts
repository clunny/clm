import { defineConfig } from 'vitepress'
import { resolve } from 'node:path'
import { guideSidebar } from './guide'

// The repository the install commands name: the one place it is written (`install-repository`).
// Until launch it is the private repository; at launch, the public one.
const installRepo = process.env.INSTALL_REPO || 'clunny/clm'

export default defineConfig({
  title: 'CLM',
  description: 'Declare the intent. AI builds the code to match. CLM keeps it matching.',
  srcDir: 'pages',
  outDir: resolve(process.env.SITE_OUT || 'dist'),
  cleanUrls: true,
  appearance: true,
  vite: {
    // A guide read from outside the package, as `GUIDE_DIR` or the repository's `docs/` says.
    server: { fs: { strict: false } },
  },
  themeConfig: {
    installRepo,
    siteTitle: 'CLM',
    nav: [
      { text: 'How it works', link: '/#how-it-works', activeMatch: '^$' },
      { text: 'Guide', link: '/guide/', activeMatch: '^/guide/(?!quickstart)' },
      { text: 'Get started', link: '/guide/quickstart' },
      { text: 'Sign in', link: '/app/', target: '_self' },
    ],
    sidebar: { '/guide/': guideSidebar() },
    outline: { level: [2, 3] },
  },
})
