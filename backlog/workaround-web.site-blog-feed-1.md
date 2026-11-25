---
title: "VitePress writes page data holding </script> unescaped into a script block"
kind: workaround
components: [web.site]
requirement: web.site/blog-feed@70788316
---

VitePress 1.6.4 writes each page's data into the page's Vue component as a JSON string inside a <script> block without escaping </script>, so the scenario 'markup in a title' (a post whose summary is x < "y" </script>) failed the build with Vue's 'Element is missing end tag'. The page's data must hold the summary as written, since VitePress's client sets the description meta from it after hydration. web/site/.vitepress/page-data.ts adds a Vite plugin that wraps VitePress's own plugin's transform (there is no hook between it and Vue's compiler) and writes each </ on the __pageData line as <\u002F, which the script reads back as </. It reaches into VitePress's resolved plugin list by name, so a VitePress upgrade that renames the plugin or escapes the string itself makes it throw or do nothing; upstream escaping would let it go.
