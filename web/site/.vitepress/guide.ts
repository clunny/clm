import { readFileSync, readdirSync } from 'node:fs'
import { dirname, join, resolve } from 'node:path'

// The directory `clm guide` prints from, read in place (`guide-from-docs`). `GUIDE_DIR` points
// tests at a fixture; `spec-format.md` is its sibling, as `docs/spec-format.md` is of `docs/guide/`.
export const guideDir = process.env.GUIDE_DIR
  ? resolve(process.env.GUIDE_DIR)
  : resolve(import.meta.dirname, '../../../docs/guide')
const specFormat = join(dirname(guideDir), 'spec-format.md')

export interface GuidePage {
  name: string // the page's name under /guide/: `index` for README.md
  file: string
}

export function guideFiles(): GuidePage[] {
  const pages = readdirSync(guideDir)
    .filter((f) => f.endsWith('.md'))
    .sort()
    .map((f) => ({ name: f === 'README.md' ? 'index' : f.slice(0, -3), file: join(guideDir, f) }))
  return [...pages, { name: 'spec-format', file: specFormat }]
}

// The guide's files link to the spec format as `../spec-format.md`; its page is a sibling.
export function guideSource(page: GuidePage): string {
  return readFileSync(page.file, 'utf8')
    .replaceAll('](../spec-format.md', '](spec-format.md')
    .replaceAll('](README.md', '](index.md')
}

// README's links in the order it gives them, then whatever it leaves unlinked, then the spec format.
export function guideSidebar(): { text: string; link: string }[] {
  const pages = guideFiles()
  const readme = pages.find((p) => p.name === 'index')
  const items: { text: string; link: string }[] = [{ text: 'Overview', link: '/guide/' }]
  const seen = new Set(['index', 'spec-format'])
  if (readme) {
    for (const m of readFileSync(readme.file, 'utf8').matchAll(/\[([^\]]+)\]\(([^)#]+)\.md\)/g)) {
      const name = m[2]
      if (seen.has(name) || !pages.some((p) => p.name === name)) continue
      seen.add(name)
      items.push({ text: m[1], link: `/guide/${name}` })
    }
  }
  for (const p of pages) {
    if (seen.has(p.name)) continue
    items.push({ text: p.name, link: `/guide/${p.name}` })
  }
  items.push({ text: 'Spec format', link: '/guide/spec-format' })
  return items
}
