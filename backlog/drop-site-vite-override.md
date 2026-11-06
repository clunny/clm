---
title: "Drop web/site's Vite override once VitePress 2 is stable"
kind: deferred
components: [web.site]
---

`web/site/package.json` overrides `vite` to 6.4.5 to clear the Dependabot alerts on the Vite 5 and esbuild that VitePress 1.6.4 requires, which puts Vite outside the range VitePress declares. When VitePress 2 has a stable release, move to it and delete the override. Until then, a further Vite advisory means raising the override within the 6.x line.
