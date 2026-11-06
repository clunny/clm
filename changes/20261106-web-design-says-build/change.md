---
status: applied
resolves: [web-design-says-build-builds-site]
---

# web's Design stops saying its build builds the site

## Intent

`web`'s Design bullet for `web/site/` (`web.site`) still says `web/package.json`'s `build` script installs the site and builds it into `web/dist/`, so `wrangler.toml`'s build command and the release's `web` job build both unchanged. Since `20261105-site-blog`, `web/app-at-app-path` says `web`'s build SHALL NOT build `web.site`, and the site is a Worker of its own that the site workflow builds and deploys (`site-worker-of-its-own`); the code already follows the requirement (`web/package.json`'s `build` runs only `tsc` and `vite build`). This change rewrites the bullet to match: the site is a package of its own, with its own `package.json`, lockfile and `wrangler.toml`, that `web`'s build leaves alone.

## Problem

A builder of `web/app-at-app-path` reads a Design that contradicts the requirement. The requirement wins, but the Design should not make the builder decide that.

## Approaches

- **Rewrite the bullet (chosen).** One Design edit, no requirement touched. Every requirement and scenario stays true and the code already matches, so nothing is left to build.
- **Drop the bullet.** The child component's own spec says what the site is, but `web`'s Design lists its three parts; leaving `web.site` out would make that list wrong in its own way.

## Outcome

`specs/web/SPEC.md`'s Design names `web/site/` as a package of its own that `web`'s build leaves alone, and says what that build does produce (the app at `web/dist/app/`, the invited list and the retiring service worker), as `web/app-at-app-path` states.

## Constraints

- Design-only: no requirement or scenario changes, so no code changes and nothing becomes pending.

No edge cases or workarounds are known.
