import type { Plugin } from 'vite'

/** A page's data, which VitePress writes into the page's Vue component as a string inside a
 * `<script>` block, may hold `</script>`: a post's summary `x < "y" </script>`, as `blog-feed`'s
 * "markup in a title" has it. VitePress writes it raw, so Vue's compiler takes it for the block's
 * end and the build fails. This plugin writes each `</` of that string as `</`, which the
 * script reads back as `</`, so the page's data, and the title and description the browser sets
 * from it, stay the text as written.
 *
 * VitePress's own plugin makes the component and Vue's compiles it next, with no hook between
 * them, so this one wraps the first's `transform` once the plugins are resolved. */
export function pageDataSafeInScript(): Plugin {
  return {
    name: 'clm:page-data-safe-in-script',
    configResolved(config) {
      const vitepress = config.plugins.find((p) => p.name === 'vitepress')
      const transform = vitepress?.transform
      if (typeof transform !== 'function') throw new Error("VitePress's plugin has no transform to wrap")
      vitepress!.transform = async function (this: unknown, code: string, id: string, options?: unknown) {
        const out = await (transform as (...a: unknown[]) => unknown).call(this, code, id, options)
        if (!id.endsWith('.md')) return out as never
        if (typeof out === 'string') return safe(out) as never
        if (out && typeof (out as { code?: unknown }).code === 'string') {
          return { ...(out as object), code: safe((out as { code: string }).code) } as never
        }
        return out as never
      } as typeof transform
    },
  }
}

/** The component's source with `</` written `</` on the line that sets its page data. */
const safe = (src: string): string =>
  src.replace(/^export const __pageData = JSON\.parse\(.*$/m, (line) => line.replaceAll('</', '<\\u002F'))
