---
title: "web's build script still builds and deploys the public site beside the app"
kind: bug
components: [web, ci]
---

`web/package.json`'s `build` runs `npm --prefix site ci` and `SITE_OUT=../dist npm --prefix site run build`, and `web/wrangler.toml` serves the result, so the release workflow's `web` job (which `ci/web-deploy` now says builds and deploys the app only) still ships a site built without `UMAMI_WEBSITE_ID` or `GITHUB_TOKEN`, and overwrites what `.github/workflows/site.yml` deploys. Also `web/site/` has no `wrangler` dev dependency or Worker config yet, which `site.yml`'s `npx wrangler deploy` (`ci/site-deploy`) needs (`site-worker-of-its-own`). Both are `web`'s and `web.site`'s to build; found by the review of the `ci` build that added `site.yml`.
