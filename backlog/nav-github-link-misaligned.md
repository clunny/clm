---
title: "The nav bar's GitHub link stacks its icon above the star count"
kind: bug
components: [web.site]
priority: P3
---

What happened: in the v0.15.0 site, the top nav's GitHub link renders its icon above the star count instead of beside it, so the icon pokes above the bar and the count sits below the other links' baseline.

Cause: the link's text in `web/site/.vitepress/config.ts` is the `gh-icon` SVG followed by `<span class="gh-count">`. VitePress's base styles set `svg { display: block }`, so the `.gh-icon` rule in `web/site/.vitepress/theme/style.css` (`vertical-align: -3px; margin-right: 5px`) has no effect and the SVG takes its own line. The homepage hero's GitHub button may be affected the same way.

Expected: icon and count on one line, vertically centred with the other nav links, as the hero button intends. Likely fix: `display: inline-block` (or make the nav link an inline flex row with `align-items: center`) on `.gh-icon`, and a Playwright check that the icon and the count share a line.
