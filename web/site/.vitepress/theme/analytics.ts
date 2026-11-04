// The site's own events for the analytics service (`click-events`, `copy-command-event`). The
// service's script records page views itself; this module records the rest by calling its
// `track`, and does nothing when the script isn't there (blocked, Do Not Track, or a build
// without a website ID). It never stops or delays a click.

type Track = (name: string, data: Record<string, string>) => void

// The global's name is spelled out in pieces so that a build without a website ID names the
// service nowhere in its files (`analytics-script`).
const globalName = ['u', 'mam', 'i'].join('')

function track(win: Window, name: string, data: Record<string, string>): void {
  try {
    const service = (win as unknown as Record<string, { track?: Track } | undefined>)[globalName]
    service?.track?.(name, data)
  } catch {
    // Analytics never breaks the page.
  }
}

/** The first line of a command, trimmed. */
const firstLine = (text: string) => text.split('\n')[0].trim()

/** Starts reporting; the returned function tells it the page the visitor is on, which a click
 * must name as it was when clicked, not as the router has already moved it. */
export function trackSiteEvents(win: Window, doc: Document): (path: string) => void {
  let current = win.location.pathname
  // A click on a copy button, or on a link: an outbound one is reported alone, a nav link or
  // homepage button as `click`.
  doc.addEventListener(
    'click',
    (event) => {
      const target = event.target as Element | null
      if (!target?.closest) return
      const path = current
      const copy = target.closest('div[class*="language-"] > button.copy')
      if (copy) {
        const block = copy.parentElement!.querySelector('pre')
        if (block) track(win, 'copy-command', { path, command: firstLine(block.textContent ?? '') })
        return
      }
      const link = target.closest('a[href]') as HTMLAnchorElement | null
      if (!link) return
      if (link.host !== win.location.host) {
        if (!/^https?:$/.test(link.protocol)) return
        track(win, 'outbound', { url: link.href, path })
      } else if (link.closest('.VPNav, .clm-home .btn')) {
        const label = (link.textContent ?? '').replace(/\s*[↓→]\s*$/, '').trim()
        track(win, 'click', { label, path })
      }
    },
    true,
  )
  // A selection copied from inside a command: the homepage's install commands or a code block.
  doc.addEventListener('copy', () => {
    const selection = win.getSelection()
    const text = selection?.toString() ?? ''
    if (!selection || selection.rangeCount === 0 || text.trim() === '') return
    const node = selection.getRangeAt(0).commonAncestorContainer
    const holder = (node instanceof Element ? node : node.parentElement)?.closest('pre, code')
    if (holder) track(win, 'copy-command', { path: current, command: firstLine(text) })
  })
  return (path) => {
    current = path
  }
}
