---
status: proposed
resolves: [cloudflare-ai-crawlers]
---

# Cloudflare lets AI crawlers through to clm-app.dev

## Intent

The operator has checked the Cloudflare zone for `clm-app.dev`: no AI crawler is blocked, **Block AI bots** is off, and Cloudflare's managed `robots.txt` is off, so the site's own `robots.txt` is the only word crawlers get. This change records that fact where `web.site`'s Design and the `site-found-by-search-and-agents` decision pointed at `backlog/cloudflare-ai-crawlers`, and deletes the item, whose steps are done.

## The human's answers

> cloudflare-ai-crawlers: Done, delete it.

## Problem

`web.site`'s Design and `docs/decisions/site-found-by-search-and-agents.md` name `backlog/cloudflare-ai-crawlers` as the place the zone's setting is kept. With the item done and deleted, those references would point at nothing, and the fact the item waited on (the zone blocks no AI crawler and adds nothing to `robots.txt`) would live only in this change's quote.

## Approaches

1. **State the zone's setting in `web.site`'s Design and amend the decision (chosen).** One sentence in each replaces the backlog reference with the setting the operator keeps.
2. **A requirement on `web.site`.** Nothing in the build can check a Cloudflare zone setting, so a requirement would be uncitable; the setting is the operator's, not code (`yagni`).

## Outcome

`web.site`'s Design says the operator keeps Cloudflare's AI crawler blocking and managed `robots.txt` off for the zone, checked on 2026-11-04. No requirement changes, so nothing is pending to build.

## Constraints and edge cases

- Cloudflare's dashboard names change (*AI Crawl Control*, formerly *Bots → AI Scrapers and Crawlers*); the Design names what the settings do, not where they sit.

## Open questions

None.
