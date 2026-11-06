---
status: applied
resolves: [web-build-still-builds-the-site]
---

# web's build stops building the public site: already designed

## Intent

Deletes `backlog/web-build-still-builds-the-site` as already fixed by the applied change `20261105-site-blog`, and files `backlog/web-design-says-build-builds-site` for a stale line of `web`'s Design.

The item asks that `web/package.json`'s `build` stop building `web.site` and that `web/site/` get a `wrangler` dev dependency and a Worker config. `20261105-site-blog` already states both:

- `web/app-at-app-path` says `web`'s build SHALL NOT build `web.site`, which deploys on its own (`site-worker-of-its-own`). The requirement is `behind` in `clm pending web`, so the next build of `web` makes the build script stop building the site; it needs nothing more from a spec.
- `web.site/site-worker` gives `web/site/wrangler.toml` and the `wrangler` dev dependency. Both are built and on `main` (`web/site/wrangler.toml`, `web/site/package.json`, cited as `req:web.site/site-worker` in `web/site/test/site.spec.ts`).

`ci/web-deploy` building the app only then holds once `web/app-at-app-path` is built.

One thing in the spec still says otherwise: `web`'s Design bullet for `web/site/` says `web/package.json`'s `build` script installs and builds the site into `web/dist/`, which contradicts `web/app-at-app-path`. That is a Design edit, not this item's fix, so it is filed as its own item for the next triage rather than designed here.
