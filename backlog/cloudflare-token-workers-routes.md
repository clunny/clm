---
title: "Give the Cloudflare API token the zone's Workers Routes permission"
kind: human-task
components: [web, ci]
origin: 20261105-site-blog
blocks: [web/site-served-directly, ci/site-deploy]
---

The app's Worker moves from the `clm-app.dev` Custom Domain to routes on the zone, and the site's Worker takes the Custom Domain (`site-worker-of-its-own`). Deploying routes needs the token's **Zone → Workers Routes → Edit** permission on `clm-app.dev`; the token in the repository's `CLOUDFLARE_API_TOKEN` secret was made from the "Edit Cloudflare Workers" template, which may or may not include it for the zone.

In the Cloudflare dashboard, under My Profile → API Tokens, edit that token: make sure its zone resources include `clm-app.dev` and its permissions include Workers Routes: Edit. If you make a new token instead, update the repository secret `CLOUDFLARE_API_TOKEN`. Then delete this item.
