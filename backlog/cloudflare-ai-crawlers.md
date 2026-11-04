---
title: "Keep Cloudflare from blocking AI crawlers on clm-app.dev"
kind: human-task
components: [web.site]
origin: 20261104-found-by-search-and-agents
---

`site-found-by-search-and-agents` welcomes every crawler in the site's `robots.txt`, but Cloudflare can block AI crawlers at the zone, in front of the site, whatever `robots.txt` says. Only the operator can change the zone's settings. Nothing in the build waits on this.

1. In the Cloudflare dashboard for `clm-app.dev`, open **AI Crawl Control** (formerly *Bots → AI Scrapers and Crawlers*) and make sure no AI crawler is blocked, and that **Block AI bots** is off.
2. Under **Security → Settings** (or *Bots*), make sure **Manage your robots.txt** / *Instruct AI bot traffic with robots.txt* is off, so Cloudflare adds nothing to the site's own `robots.txt`.
3. After the next release, open `https://clm-app.dev/robots.txt` and check that it holds only the site's one `User-agent: *` group.
4. Delete this item once that's done.
