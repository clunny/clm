---
status: applied
---

# web's build stops building the public site: already built

## Intent

Files `backlog/web-design-says-build-builds-site` for a stale line of `web`'s Design. The item this draft was given, `web-build-still-builds-the-site`, is already fixed and gone from `main`, so the change resolves nothing.

The item asked that `web/package.json`'s `build` stop building `web.site` and that `web/site/` get a `wrangler` dev dependency and a Worker config. `20261105-site-blog` states both, as `web/app-at-app-path` (`web`'s build SHALL NOT build `web.site`) and `web.site/site-worker`, and both are built: the `web` build that landed while this was drafted ("web: build the app only; the capability harness builds and serves the site") deleted the item, and `web/site/wrangler.toml` and the `wrangler` dev dependency were already on `main`.

One thing in the spec still says otherwise: `web`'s Design bullet for `web/site/` says `web/package.json`'s `build` script installs and builds the site into `web/dist/`, which contradicts `web/app-at-app-path`. That is a Design edit, filed as its own item for the next triage rather than designed here.
