---
title: "web's Design still says its build builds the site"
kind: bug
components: [web]
---

`specs/web/SPEC.md`'s Design bullet for `web/site/` (`web.site`) says `web/package.json`'s `build` script first installs the site (`npm ci --ignore-scripts`) and builds it into `web/dist/` (`SITE_OUT`), "so `wrangler.toml`'s build command and the release's `web` job build both unchanged". Since `20261105-site-blog`, `web/app-at-app-path` says `web`'s build SHALL NOT build `web.site`, and `web.site`'s Design says the site workflow builds and deploys it on its own (`site-worker-of-its-own`). The bullet should say the site is a package of its own that `web`'s build leaves alone. A builder of `web/app-at-app-path` reads the contradiction; the requirement wins. Found while drafting `20261105-web-build-still-builds`.
