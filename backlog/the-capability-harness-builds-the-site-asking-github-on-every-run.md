---
title: "The capability harness builds the site asking GitHub on every run"
kind: bug
components: [capability]
---

`capabilities/web/global-setup.ts:24` builds the site with neither `GITHUB_STARS` nor `GITHUB_API` set, so every capability run asks `api.github.com` anonymously for the star count, and can wait up to 10 seconds when GitHub is slow or unreachable. The site's own tests set one of the two. Expected: the harness's build passes `GITHUB_STARS` (any whole number), so no capability run reaches GitHub. Found by wave 6's drafter of `web.site` (`20261124-wave-6-app-by-the-rule`).
