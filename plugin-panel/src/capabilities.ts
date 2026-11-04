// The Capabilities tab (`capabilities-tab`): `clm capabilities` drawn as text, its rows
// pressable into `clm capabilities <slug>`, read-only. Rows are matched to slugs by position
// against `clm capabilities --format json`, as the inspector matches node lines; nothing here
// runs `clm capabilities decide`, which the guard asks the human about.

import type { PaneLine } from './inspector.ts'
import { type Run, type Span, ansiLines } from './panel.ts'
import { lineText } from './agents.ts'

/** The report beside the list, whose `capabilities` name the rows in order. */
const CAPABILITIES_JSON = ['capabilities', '--format', 'json']

/** Words `clm capabilities` reads as its subcommands: no page is asked for by them. */
const SUBCOMMANDS = ['decide', 'help']

/**
 * What the tab runs, on every surface: the page of the
 * capability `slug`, or the list with its report beside it.
 */
export const capabilitiesPane = (slug: string | null): { args: string[]; report?: string[] } =>
  slug === null ? { args: ['capabilities'], report: CAPABILITIES_JSON } : { args: ['capabilities', slug] }

/**
 * The list's rows by line index to slug: the lines before `unattributed` starting with `⚑ ` or
 * two spaces (the header never does), the nth being the report's nth capability, whose slug is
 * the row's first word. Null when the report didn't run or parse, or its count or a slug
 * differs, so nothing is pressable. A row slugged as a subcommand isn't pressable.
 */
export function capabilityRows(lines: readonly Span[][], report: Run | undefined): Map<number, string> | null {
  if (report === undefined || report.exitCode !== 0) return null
  let slugs: unknown[]
  try {
    const parsed = (JSON.parse(report.stdout) as { capabilities?: unknown }).capabilities
    if (!Array.isArray(parsed)) return null
    slugs = parsed.map(c => (c as { slug?: unknown } | null)?.slug)
  } catch {
    return null
  }
  const texts = lines.map(lineText)
  const end = texts.findIndex(t => t.startsWith('unattributed'))
  const at = texts.flatMap((t, i) => ((end < 0 || i < end) && /^(⚑ | {2})/.test(t) ? [i] : []))
  if (at.length !== slugs.length || at.some((line, n) => texts[line]!.slice(2).split(' ')[0] !== slugs[n])) return null
  return new Map(at.flatMap((line, n) => (SUBCOMMANDS.includes(slugs[n] as string) ? [] : [[line, slugs[n] as string] as const])))
}

export type CapabilityState = {
  /** The `capability` atom: the page asked for, or null for the list. */
  slug: string | null
  /** `clm capabilities` or `clm capabilities <slug>`. */
  run: Run
  /** `clm capabilities --format json`, run beside the list only. */
  report?: Run
}

/** What the tab draws: `Back` while a page is asked for, then `clm`'s stderr or its lines. */
export type Description = { back: boolean; error?: string; lines: PaneLine[] }

export function capabilityView(state: CapabilityState): Description {
  const back = state.slug !== null
  if (state.run.exitCode !== 0) return { back, error: state.run.stderr, lines: [] }
  const lines = ansiLines(state.run.stdout)
  if (back) return { back, lines: lines.map(spans => ({ spans })) }
  const rows = capabilityRows(lines, state.report)
  return {
    back,
    lines: lines.map((spans, i) => {
      const slug = rows?.get(i)
      return slug === undefined ? { spans } : { spans, key: `capability:${slug}` }
    }),
  }
}
