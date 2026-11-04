import DefaultTheme from 'vitepress/theme'
import type { Theme } from 'vitepress'
import { h } from 'vue'
import Home from './Home.vue'
import SiteFooter from './SiteFooter.vue'
import { trackSiteEvents } from './analytics'
import './style.css'

export default {
  extends: DefaultTheme,
  // The guide's pages have no footer of their own; `SiteFooter` adds the small one (`privacy-page`).
  Layout: () => h(DefaultTheme.Layout, null, { 'layout-bottom': () => h(SiteFooter) }),
  enhanceApp({ app, router }) {
    app.component('Home', Home)
    if (typeof window !== 'undefined') {
      const setPath = trackSiteEvents(window, document)
      const before = router.onAfterRouteChanged
      router.onAfterRouteChanged = (to) => {
        setPath(to.replace(/[?#].*$/, ''))
        return before?.(to)
      }
    }
  },
} satisfies Theme
