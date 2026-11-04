import { defineConfig } from 'vitepress'
import { resolve } from 'node:path'
import { writeFileSync } from 'node:fs'
import { guideSidebar } from './guide'
import { robotsTxt, writeAgentFiles } from './agents'
import { describePage, notFoundHtml, pageHead } from './head'
import { readStars } from './stars'

// The repository the install commands name: the one place it is written (`install-repository`).
// Until launch it is the private repository; at launch, the public one.
const installRepo = process.env.INSTALL_REPO || 'clunny/clm'

// The site's address: every absolute URL the build writes is made from it.
const siteUrl = 'https://clm-app.dev'

// The homepage's headline and its first screen's line: read by `Home.vue`, the heads and `llms.txt`.
const headline = 'Declare the intent. AI builds the code to match. CLM keeps it matching.'
const firstLine =
  'Build software solo or as a team with AI writing every line, and still have a project you can maintain.'
const description = `${headline} ${firstLine}`

const githubUrl = `https://github.com/${installRepo}`
const stars = await readStars(installRepo)

const facts = {
  siteUrl,
  installRepo,
  description,
  headline,
  umamiId: process.env.UMAMI_WEBSITE_ID ?? '',
  pagesDir: resolve(import.meta.dirname, '../pages'),
}

const githubIcon =
  '<svg class="gh-icon" viewBox="0 0 16 16" width="18" height="18" aria-hidden="true"><path fill="currentColor" d="M8 0C3.58 0 0 3.58 0 8c0 3.54 2.29 6.53 5.47 7.59.4.07.55-.17.55-.38 0-.19-.01-.82-.01-1.49-2.01.37-2.53-.49-2.69-.94-.09-.23-.48-.94-.82-1.13-.28-.15-.68-.52-.01-.53.63-.01 1.08.58 1.23.82.72 1.21 1.87.87 2.33.66.07-.52.28-.87.51-1.07-1.78-.2-3.64-.89-3.64-3.95 0-.87.31-1.59.82-2.15-.08-.2-.36-1.02.08-2.12 0 0 .67-.21 2.2.82.64-.18 1.32-.27 2-.27s1.36.09 2 .27c1.53-1.04 2.2-.82 2.2-.82.44 1.1.16 1.92.08 2.12.51.56.82 1.27.82 2.15 0 3.07-1.87 3.75-3.65 3.95.29.25.54.73.54 1.48 0 1.07-.01 1.93-.01 2.2 0 .21.15.46.55.38A8.01 8.01 0 0016 8c0-4.42-3.58-8-8-8z"/></svg>'

export default defineConfig({
  title: 'CLM',
  description,
  srcDir: 'pages',
  outDir: resolve(process.env.SITE_OUT || 'dist'),
  cleanUrls: true,
  appearance: true,
  sitemap: { hostname: siteUrl },
  vite: {
    // The social card and the like live in `web/site/public/`, beside `pages/`.
    publicDir: resolve(import.meta.dirname, '../public'),
    // A guide read from outside the package, as `GUIDE_DIR` or the repository's `docs/` says.
    server: { fs: { strict: false } },
  },
  transformPageData(pageData) {
    describePage(pageData, facts)
  },
  transformHead({ pageData }) {
    return pageHead(pageData, facts)
  },
  transformHtml(code, _id, { pageData }) {
    return pageData.isNotFound ? notFoundHtml(code) : code
  },
  buildEnd(siteConfig) {
    writeFileSync(resolve(siteConfig.outDir, 'robots.txt'), robotsTxt(siteUrl))
    writeAgentFiles(siteConfig.outDir, facts)
  },
  themeConfig: {
    installRepo,
    headline,
    firstLine,
    githubUrl,
    githubStars: stars ?? '',
    siteTitle: 'CLM',
    nav: [
      { text: 'How it works', link: '/#how-it-works', activeMatch: '^$' },
      { text: 'Guide', link: '/guide/', activeMatch: '^/guide/(?!quickstart)' },
      { text: 'Get started', link: '/guide/quickstart' },
      { text: 'Web app', link: '/app/', target: '_self' },
      {
        text: `${githubIcon}<span class="gh-count">${stars ?? 'GitHub'}</span>`,
        link: githubUrl,
        ariaLabel: stars ? `GitHub, ${stars} stars` : 'GitHub',
        noIcon: true,
      },
    ],
    sidebar: { '/guide/': guideSidebar() },
    outline: { level: [2, 3] },
  },
})
