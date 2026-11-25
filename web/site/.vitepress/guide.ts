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

/** A guide file's text for its page: its links rewritten for the page (`rewriteLinks`). */
export const guideSource = (page: GuidePage, repo: string): string =>
  rewriteLinks(readFileSync(page.file, 'utf8'), page, repo, { kind: 'page' })

/** The page's address under the site, as a path: `/guide/` for the README. */
export const pagePath = (name: string) => (name === 'index' ? '/guide/' : `/guide/${name}`)

/** A file or directory of the repository on GitHub's `main`: `tree` for a directory, `blob` for a file. */
export function repoLink(inRepo: string, endsWithSlash: boolean, anchor: string, repo: string): string {
  const onDisk = join(repoRoot, inRepo)
  const dir = endsWithSlash || (existsSync(onDisk) && statSync(onDisk).isDirectory())
  return `https://github.com/${repo}/${dir ? 'tree' : 'blob'}/main/${inRepo.replace(/\/$/, '')}${anchor}`
}

// A code span: a run of backticks, up to the next run of as many.
const CODE_SPAN = /(?<!`)(`+)[\s\S]*?(?<!`)\1(?!`)/g

/** `text` with `f` applied to each piece of it outside fenced code and code spans, so a link
 * written as code is never rewritten (`agent-markdown`). */
export function outsideCode(text: string, f: (piece: string) => string): string {
  let fenced = false
  return text
    .split('\n')
    .map((line) => {
      if (/^\s*(```|~~~)/.test(line)) {
        fenced = !fenced
        return line
      }
      if (fenced) return line
      let out = ''
      let at = 0
      for (const m of line.matchAll(CODE_SPAN)) {
        out += f(line.slice(at, m.index)) + m[0]
        at = m.index + m[0].length
      }
      return out + f(line.slice(at))
    })
    .join('\n')
}

/** The target of a relative link: its path and its `#anchor`, or undefined for a link that
 * names a scheme, an anchor alone or a path from the site's root, which stays as it is. */
export function relativeTarget(target: string): { path: string; anchor: string } | undefined {
  if (/^([a-z][a-z0-9+.-]*:|#|\/)/i.test(target)) return undefined
  const hash = target.indexOf('#')
  return hash < 0 ? { path: target, anchor: '' } : { path: target.slice(0, hash), anchor: target.slice(hash) }
}

/** On a page, `piece` with each relative image's target made the file's raw copy on GitHub's
 * `main`: the page is built from `web/site/pages/`, where no file of the guide or a post lies, so
 * left as it is the image would fail the build (`agent-markdown`). A Markdown copy keeps it. */
export function pageImages(piece: string, from: string, repo: string): string {
  return piece.replace(/(!\[[^\]]*\]\()([^)\s]+)/g, (whole, open: string, target: string) => {
    const rel = relativeTarget(target)
    if (!rel || rel.path === '') return whole
    return `${open}https://github.com/${repo}/raw/main/${posix.normalize(posix.join(from, rel.path))}${rel.anchor}`
  })
}

/** A guide file's text with the targets of its inline links outside code rewritten
 * (`agent-markdown`, `guide-pages`). A link out of the guide's files names the file on GitHub's
 * `main`. A link to another guide file names, in a Markdown copy, that file's copy, and on a page
 * that file's page, as a sibling (every guide page is under `/guide/`), so a dead guide link
 * still fails the build. */
export function rewriteLinks(
  text: string,
  page: Pick<GuidePage, 'name'>,
  repo: string,
  mode: { kind: 'page' } | { kind: 'copy'; siteUrl: string },
): string {
  // The file's place in the repository, whatever directory `GUIDE_DIR` reads it from.
  const here = page.name === 'spec-format' ? 'docs' : 'docs/guide'
  return outsideCode(text, (piece) =>
    (mode.kind === 'page' ? pageImages(piece, here, repo) : piece).replace(/(?<!!)(\[(?:!\[[^\]]*\]\([^)]*\)|[^\]])*\]\()([^)\s]+)/g, (whole, open: string, target: string) => {
      const rel = relativeTarget(target)
      if (!rel || rel.path === '') return whole
      const { path, anchor } = rel
      const inRepo = posix.normalize(posix.join(here, path))
      const guide = inRepo === 'docs/spec-format.md' ? 'spec-format.md' : /^docs\/guide\/([^/]+\.md)$/.exec(inRepo)?.[1]
      if (guide !== undefined) {
        const name = guide === 'README.md' ? 'index' : guide.slice(0, -3)
        return mode.kind === 'copy'
          ? `${open}${mode.siteUrl}/guide/${name}.md${anchor}`
          : `${open}${name}.md${anchor}`
      }
      return `${open}${repoLink(inRepo, path.endsWith('/'), anchor, repo)}`
    }),
  )
}

// README's links in the order it first gives them, labelled with their text, an `#anchor` counted
// as a link to its file; then whatever it leaves unlinked, by its title; then the spec format.
export function guideSidebar(): { text: string; link: string; name: string }[] {
  const pages = guideFiles()
  const readme = pages.find((p) => p.name === 'index')
  const items: { text: string; link: string; name: string }[] = [
    { text: 'Overview', link: '/guide/', name: 'index' },
  ]
  const seen = new Set(['index', 'spec-format'])
  if (readme) {
    for (const m of readFileSync(readme.file, 'utf8').matchAll(/\[([^\]]+)\]\(([^)#\s]+)\.md(?:#[^)\s]*)?\)/g)) {
      const name = m[2]
      if (seen.has(name) || !pages.some((p) => p.name === name)) continue
      seen.add(name)
      items.push({ text: m[1], link: `/guide/${name}`, name })
    }
  }
  for (const p of pages) {
    if (seen.has(p.name)) continue
    items.push({ text: p.title, link: `/guide/${p.name}`, name: p.name })
  }
  items.push({ text: 'Spec format', link: '/guide/spec-format', name: 'spec-format' })
  return items
}
