---
title: "Set up Umami Cloud for clm-app.dev and give the release its website ID"
kind: human-task
components: [web.site, ci]
origin: 20261104-site-analytics
---

`site-analytics-umami` needs an account only the operator can create. Nothing in the build waits on it: without the ID the released site simply reports nothing.

1. Create an Umami Cloud account (free Hobby tier) and add a website for `clm-app.dev`.
2. Copy its website ID and set it as the repository's Actions variable `UMAMI_WEBSITE_ID` (Settings → Secrets and variables → Actions → Variables). It is a variable, not a secret: it appears in every page's HTML.
3. In Umami, define the funnel: `/` → `/guide/quickstart` → event `copy-command`.
4. Delete this item once the next release's site reports page views.
