---
title: "The web/site suite runs on lands that don't affect web.site"
kind: idea
components: [ci]
origin: 20261108-land-check-timing
---

## Intent

`ci/web-checks` has `.github/scripts/ci-check.sh` run `web/site`'s suite whenever it runs `web/`'s. But `web.site` has no `depends_on` and nothing in `web` changes what it tests: `web` depends on `web.site`, not the other way round. So any land that affects `web` pays for the site's suite. On 2026-11-08 a one-test change to `clm-web-replica` (`web.worker.replica`) affected `web.worker` and then `web`, and spent 81 s of its 218 s check in `web/site`, a suite it couldn't break.

Run `web/site` only when the selection names a file under `web/site/` (or when every test runs), as `capabilities/web/` and `capabilities/panel/` are selected by their own files. That needs `ci/web-checks` reworded.

## Open questions

- Should `capabilities/web/` keep running whenever `web/` runs? It drives the Worker as deployed, so a Worker change can break it, but a change only to `web/`'s browser app may not need it. `20261108-land-check-timing`'s step times and chains are the evidence to judge it by.
- The same land also selected all of `web/src` (40 files) through `web`'s ownership, since those files carry no `req:` marker. Once step times are recorded, judge whether that reach is worth narrowing too.
