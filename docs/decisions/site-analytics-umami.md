---
title: "The public site's analytics: Umami Cloud, cookieless, only in the deployed build"
date: 2026-11-04
status: accepted
---

The public site (`web.site`) measures how newcomers use it (`see-site-use`): page views, referrers, clicks and copied commands, and the funnel from the homepage to a copied install command.

**Umami Cloud** (`reuse-first`). It records custom events and builds funnels, sets no cookie and stores no IP address, and its free Hobby tier covers the site's traffic. Rejected: Cloudflare Web Analytics, which counts pages only and can't record a copied command; collecting into Workers Analytics Engine through our Worker, which means building the collection, the queries and a funnel view a standard tool has, and spends a Worker request per event where the site's pages spend none (`site-served-from-assets`); Plausible, equivalent but paid for funnels. Moving to Plausible later changes only the script tag and the theme's one tracking module.

**Our code calls `umami.track`**, rather than relying on Umami's `data-umami-event` attributes, so the events are ours to test with Umami's script stubbed, and a blocked script is a no-op.

**Only the deployed site reports.** The website ID reaches the build only through `UMAMI_WEBSITE_ID`, which the site workflow a human dispatches, `.github/workflows/site.yml`, sets from an Actions variable when it builds and deploys the site's own Worker (`site-worker-of-its-own`, `ci/site-deploy`). Tests, local builds and forks hold no analytics, and the operator's ID isn't in the repository (as `operator-ids-out-of-the-repo` wants for the others).

**Blocked visitors are not chased.** Developers block trackers often, so Umami undercounts. Serving its script through our own domain to get past blockers was rejected: a visitor who blocks trackers has chosen not to be tracked. Do Not Track is honoured for the same reason. The undercount is judged against Cloudflare's own traffic numbers for the zone, counted at the edge with no script, read in its dashboard.

**Only the site.** The app under `/app/` is used by invited people only and isn't measured. How CLM is used after install, in people's own repositories, is a separate question (`backlog/cli-usage-reporting`).
