---
title: "A relative image on a guide or post page fails the build, so pages point it at GitHub"
kind: workaround
components: [web.site]
requirement: web.site/agent-markdown@e6e9273a
---

The scenario 'links into the repository' has a post holding the image ![pic](next.md) and needs the build to succeed. A post's page (and a guide page) is built from web/site/pages/, where no file of the post or the guide lies, so VitePress resolves a relative image as an import there and fails the build ('Could not resolve ./next.md'). The requirement states what the Markdown copy does with an image (keeps it) but not what the page does. The build makes a relative image on a page name the file's raw copy on GitHub's main (https://github.com/<repo>/raw/main/<path>), the same rule as the pages' links into the repository, in pageImages (web/site/.vitepress/guide.ts). A spec change could state it, or say the site serves the blog's and the guide's images itself.
