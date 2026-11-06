import { existsSync, readFileSync } from 'node:fs'
import { join } from 'node:path'
import type { HeadConfig, PageData } from 'vitepress'
import { guideDir, guideFiles, plain, titleAndSummary } from './guide'
import type { SiteFacts } from './agents'
import { publishedPosts } from './blog'
import type { Post } from './blog'

export interface HeadFacts extends SiteFacts {
  headline: string
  umamiId: string
  pagesDir: string // where the site's own Markdown pages (privacy) live
}

/** The page's clean URL path: `/`, `/guide/`, `/guide/quickstart`, `/privacy`. */
export function urlPath(relativePath: string): string {
  const p = '/' + relativePath.replace(/\.md$/, '')
  return p === '/index' ? '/' : p.endsWith('/index') ? p.slice(0, -5) : p
}

/** A guide page's name under `/guide/`, from its path, or undefined for any other page. */
function guideName(relativePath: string): string | undefined {
  const m = /^guide\/(.+)\.md$/.exec(relativePath)
  return m ? m[1] : undefined
}

/** The published post a page is, from its path (`blog/<slug>.md`), or undefined for any other page. */
function postOf(relativePath: string): Post | undefined {
  const m = /^blog\/(.+)\.md$/.exec(relativePath)
  return m ? publishedPosts().find((p) => p.slug === m[1]) : undefined
}

/** Sets a page's title and description before it renders (`page-metadata`). */
export function describePage(pageData: PageData, facts: HeadFacts): void {
  const { relativePath } = pageData
  if (relativePath === 'index.md') {
    pageData.title = 'CLM: declare the intent, AI builds the code to match'
    pageData.titleTemplate = false
    pageData.description = facts.description
    return
  }
  const name = guideName(relativePath)
  if (name !== undefined) {
    const page = guideFiles().find((p) => p.name === name)
    if (!page) return
    pageData.title = name === 'index' ? 'CLM guide' : page.title
    pageData.titleTemplate = name === 'index' ? false : ':title | CLM guide'
    pageData.description = page.summary
    return
  }
  if (relativePath === 'blog/index.md') {
    pageData.title = 'CLM blog'
    pageData.titleTemplate = false
    pageData.description = facts.blogDescription
    return
  }
  const post = postOf(relativePath)
  if (post) {
    pageData.title = post.title
    pageData.titleTemplate = ':title | CLM blog'
    pageData.description = post.summary
    return
  }
  const own = join(facts.pagesDir, relativePath)
  if (existsSync(own)) {
    const { title, summary } = titleAndSummary(readFileSync(own, 'utf8'))
    pageData.title = `${title ?? relativePath} | CLM`
    pageData.titleTemplate = false
    if (summary) pageData.description = summary
  }
}

/** The 404 page's own title and description: VitePress builds that page without running
 * `transformPageData`, so its HTML is rewritten after (`page-metadata`). */
export function notFoundHtml(html: string): string {
  return html
    .replace(/<title>[^<]*<\/title>/, '<title>Page not found | CLM</title>')
    .replace(/<meta name="description" content="[^"]*">/, '<meta name="description" content="This page does not exist.">')
}

const meta = (attrs: Record<string, string>): HeadConfig => ['meta', attrs]

/** The `<script type="application/ld+json">` of a page that has structured data. */
const jsonLd = (data: unknown): HeadConfig => [
  'script',
  { type: 'application/ld+json' },
  JSON.stringify(data).replaceAll('<', '\\u003c'),
]

/** The questions of the FAQ file: each `##` heading and the text under it (`structured-data`). */
export function faqEntries(text: string): { question: string; answer: string }[] {
  const entries: { question: string; body: string[] }[] = []
  let fenced = false
  for (const line of text.split('\n')) {
    if (/^\s*(```|~~~)/.test(line)) fenced = !fenced
    const heading = !fenced && /^## (.+)$/.exec(line)
    if (heading) entries.push({ question: plain(heading[1]), body: [] })
    else if (entries.length > 0) entries[entries.length - 1].body.push(line)
  }
  return entries.map((e) => ({
    question: e.question,
    answer: e.body
      .join('\n')
      .split(/\n\s*\n/)
      .map((para) => plain(para.replace(/^\s*([-*+]|\d+[.)])\s+/gm, '').replace(/^\s*(```|~~~).*$/gm, '')))
      .filter((para) => para !== '')
      .join('\n'),
  }))
}

/** What every page's head holds beyond VitePress's own: the canonical link, the social tags, the
 * Markdown alternate, structured data and, when the build has an ID, the analytics script. */
export function pageHead(pageData: PageData, facts: HeadFacts): HeadConfig[] {
  const { siteUrl, installRepo, description, umamiId } = facts
  const head: HeadConfig[] = []
  if (umamiId !== '') {
    head.push([
      'script',
      {
        defer: '',
        src: 'https://cloud.umami.is/script.js',
        'data-website-id': umamiId,
        'data-do-not-track': 'true',
      },
    ])
  }
  if (pageData.isNotFound) {
    head.push(meta({ name: 'robots', content: 'noindex' }))
    return head
  }
  const { relativePath, description: pageDescription = '' } = pageData
  const template = pageData.titleTemplate
  const title =
    typeof template === 'string' ? template.replace(':title', pageData.title) : pageData.title
  const url = siteUrl + urlPath(relativePath)
  head.push(['link', { rel: 'canonical', href: url }])
  const social: [string, string, string][] = [
    ['property', 'og:title', title],
    ['property', 'og:description', pageDescription],
    ['property', 'og:url', url],
    ['property', 'og:type', postOf(relativePath) ? 'article' : 'website'],
    ['property', 'og:site_name', 'CLM'],
    ['property', 'og:image', `${siteUrl}/social-card.png`],
    ['name', 'twitter:card', 'summary_large_image'],
    ['name', 'twitter:title', title],
    ['name', 'twitter:description', pageDescription],
    ['name', 'twitter:image', `${siteUrl}/social-card.png`],
  ]
  for (const [key, name, content] of social) head.push(meta({ [key]: name, content }))

  const name = guideName(relativePath)
  if (name !== undefined) {
    head.push(['link', { rel: 'alternate', type: 'text/markdown', href: `${siteUrl}/guide/${name}.md` }])
  }
  const post = postOf(relativePath)
  const blogIndex = relativePath === 'blog/index.md'
  if (post || blogIndex) {
    head.push(['link', { rel: 'alternate', type: 'application/rss+xml', title: 'CLM blog', href: `${siteUrl}/blog/feed.xml` }])
  }
  if (post) {
    head.push(['link', { rel: 'alternate', type: 'text/markdown', href: `${siteUrl}/blog/${post.slug}.md` }])
    head.push(meta({ property: 'article:published_time', content: post.date }))
    head.push(
      jsonLd({
        '@context': 'https://schema.org',
        '@type': 'BlogPosting',
        headline: post.title,
        description: post.summary,
        datePublished: post.date,
        keywords: post.tags,
        url: url,
        author: { '@type': 'Organization', name: 'CLM' },
        publisher: { '@type': 'Organization', name: 'CLM' },
      }),
    )
  }
  if (blogIndex) {
    head.push(
      jsonLd({
        '@context': 'https://schema.org',
        '@type': 'Blog',
        name: 'CLM blog',
        description: pageDescription,
        url,
        blogPost: publishedPosts().map((p) => ({
          '@type': 'BlogPosting',
          headline: p.title,
          datePublished: p.date,
          url: `${siteUrl}/blog/${p.slug}`,
        })),
      }),
    )
  }
  if (relativePath === 'index.md') {
    head.push(
      jsonLd({
        '@context': 'https://schema.org',
        '@type': 'SoftwareApplication',
        name: 'CLM',
        description,
        url: `${siteUrl}/`,
        applicationCategory: 'DeveloperApplication',
        operatingSystem: 'Linux, macOS',
        offers: { '@type': 'Offer', price: '0', priceCurrency: 'USD' },
        license: 'https://opensource.org/licenses/MIT',
        sameAs: [`https://github.com/${installRepo}`],
      }),
    )
  }
  if (name === 'faq' && existsSync(join(guideDir, 'faq.md'))) {
    const entries = faqEntries(readFileSync(join(guideDir, 'faq.md'), 'utf8'))
    if (entries.length > 0) {
      head.push(
        jsonLd({
          '@context': 'https://schema.org',
          '@type': 'FAQPage',
          mainEntity: entries.map((e) => ({
            '@type': 'Question',
            name: e.question,
            acceptedAnswer: { '@type': 'Answer', text: e.answer },
          })),
        }),
      )
    }
  }
  return head
}
