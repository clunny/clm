import { mkdirSync, readFileSync, writeFileSync } from 'node:fs'
import { join } from 'node:path'
import { guideFiles, guideSidebar, pagePath, rewriteLinks } from './guide'

export interface SiteFacts {
  siteUrl: string
  installRepo: string
  description: string
}

/** `robots.txt`: one group for every crawler, disallowing the app's paths (`crawler-files`). */
export function robotsTxt(siteUrl: string): string {
  return `User-agent: *\nDisallow: /app/\nDisallow: /api/\nDisallow: /auth/\n\nSitemap: ${siteUrl}/sitemap.xml\n`
}

/** The Markdown copies, `llms.txt` and `llms-full.txt`, made from the guide's files at build time
 * and never committed (`agent-markdown`). */
export function writeAgentFiles(outDir: string, facts: SiteFacts): void {
  const { siteUrl, installRepo, description } = facts
  const pages = guideFiles()
  const copies = new Map<string, string>()
  mkdirSync(join(outDir, 'guide'), { recursive: true })
  for (const page of pages) {
    const copy = rewriteLinks(readFileSync(page.file, 'utf8'), page, siteUrl, installRepo)
    copies.set(page.name, copy)
    writeFileSync(join(outDir, 'guide', `${page.name}.md`), copy)
  }
  const ordered = guideSidebar().map((item) => pages.find((p) => p.name === item.name)!)

  const entries = ordered.map(
    (p) => `- [${p.title}](${siteUrl}/guide/${p.name}.md): ${p.summary}`,
  )
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

## Optional

- [Full guide text](${siteUrl}/llms-full.txt): every guide page in one file
- [GitHub repository](https://github.com/${installRepo})
`
  writeFileSync(join(outDir, 'llms.txt'), llms)

  const full = ordered
    .map((p) => `<!-- ${siteUrl}${pagePath(p.name)} -->\n${copies.get(p.name)!.replace(/\n+$/, '')}\n`)
    .join('\n---\n\n')
  writeFileSync(join(outDir, 'llms-full.txt'), full)
}
