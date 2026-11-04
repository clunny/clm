// The engine's globals a hooks module has: what its JSX compiles against.
type Element = { type: unknown; props: Record<string, unknown>; children: unknown[] }
const g = globalThis as Record<string, unknown>
g.h = (type: unknown, props: Record<string, unknown> | null, ...children: unknown[]): Element => ({
  type,
  props: props ?? {},
  children,
})
g.Fragment = 'Fragment'
