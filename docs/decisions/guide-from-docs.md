---
title: "The user guide is Markdown in docs/guide/, bundled into the web app"
date: 2026-10-22
status: accepted
---

CLM's user guide is `docs/guide/*.md` in this repository: the only copy. The web app's build bundles those files and shows them in a Guide view. CI deploys the app on release (`ci`), so the web shows the guide of the release people run, matching their plugin and `clm`. A test fails when a page isn't in the bundle.

`docs/` is a spec path, so the guide changes through spec PRs, reviewed like the specs it explains. It describes CLM as released, not plans: future work stays in `backlog/`.

Rejected: the Worker fetching the guide from this repository at run time. The repository is private, so the App would need installing on it; each view would spend the Free plan's request budget (`web-on-free-plan`); and readers would get `main`'s guide, which may be ahead of their release.

**Amended 2026-10-29** (`20261029-refounding`). The web app no longer has a Guide view: `web/guide-view` was cut at the re-founding's mapping, since `clm guide` already prints the guide at the version that runs (`guide-in-the-binary`) and no capability needs it in a browser. The guide stays `docs/guide/*.md`, the only copy, changed through spec branches.

**Amended 2026-11-01** (`20261101-public-site`). The guide is back on the web, as pages of CLM's public site at `/guide/`, built by VitePress in `web.site` from `docs/guide/*.md` in place, not as a view of the app (`public-site`). It is still the only copy, deployed on release, so the web shows the released guide.
