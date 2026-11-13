---
title: "The web app is invite-only, and the list is in the web Contract"
date: 2026-10-22
status: accepted
---

Refines `web-github-app`. Anyone may still install the App, but only the GitHub logins listed in `web`'s Contract may sign in. Everyone else gets a "not invited" page and no session, and nothing of the app is public, including the guide. There is no sign-up.

The list is in the Contract, not in a code file such as `web/wrangler.toml`, so there's one copy: the Worker reads it from `specs/web/SPEC.md` when it is built. It was put there also so that adding someone would be a HARD spec PR a human approves (`humans-review-intent`); since `yes-no-review` that no longer holds (see the amendment below).

Revisit when CLM opens to other teams: sign-up, or access by App installation alone, would replace the list.

(Amended 2026-10-29, `20261029-refounding`: since `yes-no-review`, a spec edit needs review only when its change declares a burden, so adding a login to the list lands on checks. The human accepted that at the re-founding's second touchpoint: the list stays in `web`'s Contract, so the Worker keeps one copy, and adding someone no longer waits for a human.)

(Amended 2026-11-01, `20261101-public-site`: the public site at `clm-app.dev`'s root is part of the web app and stays behind the invited list until the human publishes it (`backlog/publish-the-site`). Then only the site's pages open to everyone, this decision is amended again, and the app stays invite-only.)

(Amended 2026-11-03, `20261103-publish-the-site`: the human published the site. Its pages are open to everyone, served by the assets with no Worker code (`site-served-from-assets`). The list still decides who signs in to the app under `/app/` and reaches its API, and the guide is part of the site, so it is public too.)

## The rule in full

`ARCHITECTURE.md` stated this as the entry `invite-only-web` until 2026-11-13, when the entry was cut to its rule (`human-layer-entries-stay-short`). The text still holds:

Only the logins listed in `web`'s Contract SHALL sign in to the web app under `/app/` and reach its API; nothing it shows of a repository is public. The public site (`web.site`) is open to everyone and shows none of the repository data the web app keeps behind sign-in: no repository's specs, backlog or changes. CLM's own documentation, which it builds from CLM's repository, is public by design, and it may name and link repositories, such as CLM's own in its install commands. Adding a login edits that list, and lands on checks like any change without a burden.

**Scenario: stranger signs in**
- GIVEN a GitHub login not on the list
- WHEN it signs in to the web app
- THEN it is refused and sees nothing

**Scenario: stranger opens the site**
- GIVEN someone with no session
- WHEN they open the homepage and the guide, and then ask the API for the repo list
- THEN they read the site, and the API answers 401
