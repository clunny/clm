---
title: "The user guide is Markdown in docs/guide/, the only copy, printed by clm guide and published on the public site"
date: 2026-10-22
status: accepted
---

CLM's user guide is `docs/guide/*.md` in this repository: the only copy. It reaches readers two ways, both built from those files in place:
- `clm guide` prints it, offline and at the version that runs (`guide-in-the-binary`).
- CLM's public site shows it at `/guide/`, built by VitePress in `web.site` (`public-site`). The site is built from `main` and deployed by the site workflow a human dispatches (`site-worker-of-its-own`), so the web shows the guide as of that deploy.

The web app itself has no Guide view: `clm guide` already prints the guide at the version that runs, and no capability needs it inside the app.

`docs/` is a spec path, so the guide changes on spec branches, on the same terms as the specs it explains (`yes-no-review`); an edit that only rewords or adds a page needs no change (`small-change-no-claim`). It describes CLM as released, not plans: future work stays in `backlog/`.

Rejected:
- The Worker fetching the guide from this repository at run time. The App would need installing on the repository, and each view would spend the Free plan's request budget (`web-on-free-plan`).
- The guide as a view inside the app. Slow first paint, unreadable by search engines, and behind the app's sign-in (`public-site`, `invite-only-web`).
