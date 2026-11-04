import { existsSync, readFileSync, readdirSync, statSync } from 'node:fs'
import { dirname, join, posix, resolve } from 'node:path'

// The directory `clm guide` prints from, read in place (`guide-from-docs`). `GUIDE_DIR` points
// tests at a fixture; `spec-format.md` is its sibling, as `docs/spec-format.md` is of `docs/guide/`.
export const guideDir = process.env.GUIDE_DIR
  ? resolve(process.env.GUIDE_DIR)
  : resolve(import.meta.dirname, '../../../docs/guide')
const specFormat = join(dirname(guideDir), 'spec-format.md')
// The repository's top, for telling a file from a directory in a link into the repository.
const repoRoot = resolve(import.meta.dirname, '../../..')

export interface GuidePage {
  name: string // the page's name under /guide/: `index` for README.md
  file: string
  title: string // the file's `#` title
  summary: string // the paragraph after it, as plain text (`page-metadata`)
}

/** Markdown reduced to the text a reader sees: links, code, emphasis and images lose their marks. */
export function plain(md: string): string {
  return md
    .replace(/!\[([^\]]*)\]\([^)]*\)/g, '$1')
    .replace(/\[([^\]]+)\]\([^)]*\)/g, '$1')
    .replace(/`([^`]+)`/g, '$1')
    .replace(/(\*\*|__)(.+?)\1/g, '$2')
    .replace(/\*([^*\n]+)\*/g, '$1')
    .replace(/(^|\W)_([^_\n]+)_(?=\W|$)/g, '$1$2')
    .replace(/\s+/g, ' ')
    .trim()
}

const NOT_A_PARAGRAPH = /^(\s*([-*+]|\d+[.)])\s|\s*#|\s*```|\s*~~~|\s*>|\s*\||\s*<|\s{4})/

/** The title and the summary paragraph of a page's text; `summary` is undefined when the first
 * block after the title isn't a paragraph. */
export function titleAndSummary(text: string): { title?: string; summary?: string } {
  const lines = text.split('\n')
  const at = lines.findIndex((l) => /^# /.test(l))
  if (at < 0) return {}
  const title = lines[at].slice(2).trim()
  let i = at + 1
  while (i < lines.length && lines[i].trim() === '') i++
  if (i >= lines.length || NOT_A_PARAGRAPH.test(lines[i])) return { title }
  const block: string[] = []
  for (; i < lines.length && lines[i].trim() !== ''; i++) block.push(lines[i])
  return { title, summary: plain(block.join(' ')) }
}

export function guideFiles(): GuidePage[] {
  const pages = readdirSync(guideDir)
    .filter((f) => f.endsWith('.md'))
    .sort()
    .map((f) => ({ name: f === 'README.md' ? 'index' : f.slice(0, -3), file: join(guideDir, f) }))
  return [...pages, { name: 'spec-format', file: specFormat }].map((p) => {
    const { title, summary } = titleAndSummary(readFileSync(p.file, 'utf8'))
    if (title === undefined) throw new Error(`${p.file}: the guide file has no # title`)
    if (summary === undefined) {
      throw new Error(`${p.file}: the first block after the title must be a summary paragraph`)
    }
    return { ...p, title, summary }
  })
}

// The guide's files link to the spec format as `../spec-format.md`; its page is a sibling.
export function guideSource(page: GuidePage): string {
  return readFileSync(page.file, 'utf8')
    .replaceAll('](../spec-format.md', '](spec-format.md')
    .replaceAll('](README.md', '](index.md')
}

/** The page's address under the site, as a path: `/guide/` for the README. */
export const pagePath = (name: string) => (name === 'index' ? '/guide/' : `/guide/${name}`)

/** The file's text for its Markdown copy: every inline link outside fenced code to another guide
 * file names that file's copy, and every other relative link names the file on GitHub's `main`. */
export function rewriteLinks(text: string, page: GuidePage, siteUrl: string, repo: string): string {
  const isSpecFormat = page.name === 'spec-format'
  // The file's place in the repository, whatever directory `GUIDE_DIR` reads it from.
  const here = isSpecFormat ? 'docs' : 'docs/guide'
  let fenced = false
  return text
    .split('\n')
    .map((line) => {
      if (/^\s*(```|~~~)/.test(line)) fenced = !fenced
      if (fenced) return line
      return line.replace(/(\]\()([^)\s]+)/g, (whole, open: string, target: string) => {
        if (/^([a-z][a-z0-9+.-]*:|#|\/\/)/i.test(target)) return whole
        const hash = target.indexOf('#')
        const path = hash < 0 ? target : target.slice(0, hash)
        const anchor = hash < 0 ? '' : target.slice(hash)
        if (path === '') return whole
        const inRepo = posix.normalize(posix.join(here, path))
        if (inRepo === 'docs/spec-format.md') return `${open}${siteUrl}/guide/spec-format.md${anchor}`
        const guide = /^docs\/guide\/([^/]+\.md)$/.exec(inRepo)
        if (guide) {
          const name = guide[1] === 'README.md' ? 'index' : guide[1].slice(0, -3)
          return `${open}${siteUrl}/guide/${name}.md${anchor}`
        }
        const onDisk = join(repoRoot, inRepo)
        const dir = path.endsWith('/') || (existsSync(onDisk) && statSync(onDisk).isDirectory())
        return `${open}https://github.com/${repo}/${dir ? 'tree' : 'blob'}/main/${inRepo.replace(/\/$/, '')}${anchor}`
      })
    })
    .join('\n')
}

// README's links in the order it gives them, then whatever it leaves unlinked, then the spec format.
export function guideSidebar(): { text: string; link: string; name: string }[] {
  const pages = guideFiles()
  const readme = pages.find((p) => p.name === 'index')
  const items: { text: string; link: string; name: string }[] = [
    { text: 'Overview', link: '/guide/', name: 'index' },
  ]
  const seen = new Set(['index', 'spec-format'])
  if (readme) {
    for (const m of readFileSync(readme.file, 'utf8').matchAll(/\[([^\]]+)\]\(([^)#]+)\.md\)/g)) {
      const name = m[2]
      if (seen.has(name) || !pages.some((p) => p.name === name)) continue
      seen.add(name)
      items.push({ text: m[1], link: `/guide/${name}`, name })
    }
  }
  for (const p of pages) {
    if (seen.has(p.name)) continue
    items.push({ text: p.name, link: `/guide/${p.name}`, name: p.name })
  }
  items.push({ text: 'Spec format', link: '/guide/spec-format', name: 'spec-format' })
  return items
}
