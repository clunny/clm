---
title: "The public site is served by the assets, with no Worker request"
date: 2026-11-03
status: accepted
---

Refines `public-site` and `invite-only-web`. Once the site is open to everyone, nothing decides who sees it, so the Worker has no reason to run for it: `web/wrangler.toml`'s `run_worker_first` sends only `/api/*`, `/auth/*`, `/webhook`, `/_clm/*` and `/app` to the Worker, and the assets answer the rest, the site's missing pages included (`not_found_handling = "404-page"`).

- Static asset requests that invoke no Worker are free and unmetered on Cloudflare, while the Free plan allows 100,000 Worker requests a day (`web-on-free-plan`). A public page, and any crawler, spends none of that allowance, so traffic to the site can't take the app down.
- The assets' own `Cache-Control` and `ETag` reach the browser, and there's no `private` to set and no session to renew on a public page.
- The Worker keeps answering `/_clm/*` with 404, so the invited list stays unserved, and redirecting `/app`.

Rejected: keeping the site's paths on the Worker and dropping the session check. It is one line less to change, but spends a Worker request on every public page view and leaves site code in the Worker for nothing (`no-dead-code`).

Kept: `web.worker.platform/asset-responses`'s `fetch_asset`, now reached only through `asset(path)` for the invited list. Going back to a text-only `asset` would rewrite the adapter and the test fixtures again for no change in behaviour.
