---
title: "The public site is a Cloudflare Worker of its own, deployed by a workflow the human dispatches"
date: 2026-11-05
status: accepted
---

Refines `public-site`, which put the site and the app in one Worker deployed by the release. A blog released one post at a time (`publish-posts`) needs the site to go out without a CLM release, and the human wants the two released independently.

**Two Workers on one hostname.** The site is an assets-only Worker, `clm-site` (`web.site/site-worker`), holding the `clm-app.dev` Custom Domain. The app's Worker, `clm` (`web`), drops the Custom Domain and claims the app's paths as routes on the same zone: `/app`, `/app/*`, `/api/*`, `/auth/*`, `/webhook`, `/_clm/*` and `/sw.js` (`web/site-served-directly`). Cloudflare runs a route's Worker before the Custom Domain's ("Interaction with Routes" in Cloudflare's Custom Domains documentation), so those paths never reach the site, and everything else does. The site's Worker has no script, so a request to the site still runs no code and spends no Worker request, as before. `invite-only-web` holds: every path that reaches a repository's data is the app's.

**Two pipelines.** The release workflow's `web` job deploys the app only (`ci/web-deploy`). `.github/workflows/site.yml`, dispatched by the human, builds `web/site/` from `main` and deploys the site (`ci/site-deploy`), with the Umami website ID and the star-count token the release used to pass. It runs on Actions for the reason the release does (`host-ci-for-releases`): it holds the Cloudflare secrets, which this machine otherwise wouldn't. The guard treats it as it treats the release (`plugin.hooks.guard/release-guard`, `humans-cut-releases`): publishing the site is outward-facing, so only the human dispatches it.

**The move.** It happens once on the live zone, and the order matters: the app's routes are deployed before the site's Worker takes the Custom Domain, so `/app/` and the API are never answered by the site. Deploying the routes needs the token in the secrets to hold the zone's Workers Routes permission. It does: on 2026-11-05 the human edited the token behind `CLOUDFLARE_API_TOKEN` in place to hold Zone → Workers Routes → Edit and Zone → Zone → Read, with `clm-app.dev` among its zone resources, so the secret itself didn't change (`20261105-cloudflare-token-workers-routes`).

Alternatives considered:
- *A deploy workflow of its own for the one Worker.* Smallest, but every site deploy also ships the app from `main`, and the two can't be released apart.
- *Scheduled publishing on top of that.* A daily deploy publishing posts whose date has passed would release the series by itself, at the cost of a scheduled Actions run and posts going live unwatched. It can be added later if wanted.
- *Cloudflare Pages for the site.* Same split, but a second Cloudflare product, and a Pages project and a Worker on one hostname route less predictably than two Workers.
