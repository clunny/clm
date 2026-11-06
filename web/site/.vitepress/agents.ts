import { mkdirSync, readFileSync, writeFileSync } from 'node:fs'
import { join } from 'node:path'
import { guideFiles, guideSidebar, pagePath, rewriteLinks } from './guide'
import { loadPosts, postCopy, publishedPosts } from './blog'

export interface SiteFacts {
  siteUrl: string
  installRepo: string
  description: string
  blogDescription: string
}

const xml = (s: string) =>
  s.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;').replace(/"/g, '&quot;')

/** `/blog/feed.xml`: RSS 2.0, one item for each published post, newest first (`blog-feed`). */
export function feedXml(siteUrl: string, blogDescription: string): string {
  const items = publishedPosts().map((p) => {
    const url = `${siteUrl}/blog/${p.slug}`
    const pubDate = new Date(`${p.date}T00:00:00Z`).toUTCString()
    return `    <item>
      <title>${xml(p.title)}</title>
      <link>${url}</link>
      <guid>${url}</guid>
      <description>${xml(p.summary)}</description>
      <pubDate>${pubDate}</pubDate>
    </item>
`
  })
  return `<?xml version="1.0" encoding="UTF-8"?>
<rss version="2.0">
  <channel>
    <title>CLM blog</title>
    <link>${siteUrl}/blog/</link>
    <description>${xml(blogDescription)}</description>
${items.join('')}  </channel>
</rss>
`
}

/** `robots.txt`: one group for every crawler, disallowing the app's paths (`crawler-files`). */
export function robotsTxt(siteUrl: string): string {
  return `User-agent: *\nDisallow: /app/\nDisallow: /api/\nDisallow: /auth/\n\nSitemap: ${siteUrl}/sitemap.xml\n`
}

/** The Markdown copies, `llms.txt` and `llms-full.txt`, made from the guide's files at build time
 * and never committed (`agent-markdown`). */
export function writeAgentFiles(outDir: string, facts: SiteFacts): void {
  const { siteUrl, installRepo, description, blogDescription } = facts
  const pages = guideFiles()
  const copies = new Map<string, string>()
  mkdirSync(join(outDir, 'guide'), { recursive: true })
  for (const page of pages) {
    const copy = rewriteLinks(readFileSync(page.file, 'utf8'), page, siteUrl, installRepo)
    copies.set(page.name, copy)
    writeFileSync(join(outDir, 'guide', `${page.name}.md`), copy)
  }
  const all = loadPosts()
  const posts = publishedPosts(all)
  mkdirSync(join(outDir, 'blog'), { recursive: true })
  for (const post of posts) {
    writeFileSync(join(outDir, 'blog', `${post.slug}.md`), postCopy(post, all, siteUrl, installRepo))
  }
  writeFileSync(join(outDir, 'blog', 'feed.xml'), feedXml(siteUrl, blogDescription))
  const ordered = guideSidebar().map((item) => pages.find((p) => p.name === item.name)!)

  const entries = ordered.map(
    (p) => `- [${p.title}](${siteUrl}/guide/${p.name}.md): ${p.summary}`,
  )
  const blog =
    posts.length === 0
      ? ''
      : `## Blog

${posts.map((p) => `- [${p.title}](${siteUrl}/blog/${p.slug}.md): ${p.summary}`).join('\n')}

`
  const llms = `# CLM

> ${description}

CLM is a Claude Code plugin and a \`clm\` CLI for building software with AI from a spec. To install it, run these in Claude Code:

\`\`\`
/plugin marketplace add ${installRepo}
/plugin install clm@clm
/clm:init
\`\`\`

## Guide

${entries.join('\n')}

${blog}## Optional

- [Full guide text](${siteUrl}/llms-full.txt): every guide page in one file
- [GitHub repository](https://github.com/${installRepo})
`
  writeFileSync(join(outDir, 'llms.txt'), llms)

  const full = ordered
    .map((p) => `<!-- ${siteUrl}${pagePath(p.name)} -->\n${copies.get(p.name)!.replace(/\n+$/, '')}\n`)
    .join('\n---\n\n')
  writeFileSync(join(outDir, 'llms-full.txt'), full)
}
