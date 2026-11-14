---
title: "The web app is invite-only, and the list is in the web Contract"
date: 2026-10-22
status: accepted
---

Refines `web-github-app`. Anyone may still install the App, but only the GitHub logins listed in `web`'s Contract may sign in to the web app under `/app/` and reach its API. Everyone else gets a "not invited" page and no session, and nothing of the app is public. There is no sign-up; someone who isn't invited asks through an issue form (`early-access-by-issue-form`).

The public site at `clm-app.dev`'s root is open to everyone: the human published it, and its pages are served with no Worker code (`site-served-from-assets`, `site-worker-of-its-own`). The guide is part of the site, so it is public too. Every path that reaches a repository's data is the app's, behind the list.

The list is in the Contract, not in a code file such as `web/wrangler.toml`, so there's one copy: the Worker reads it from `specs/web/SPEC.md` when it is built. Adding a login is a spec edit whose change declares no burden, so it lands on checks and doesn't wait for a human (`yes-no-review`); the human accepted that when review became yes or no, rather than move the list.

Revisit when CLM opens to other teams: sign-up, or access by App installation alone, would replace the list.

## The rule in full

`ARCHITECTURE.md`'s entry `invite-only-web` states this in short; the rule in full:

Only the logins listed in `web`'s Contract SHALL sign in to the web app under `/app/` and reach its API; nothing it shows of a repository is public. The public site (`web.site`) is open to everyone and shows none of the repository data the web app keeps behind sign-in: no repository's specs, backlog or changes. CLM's own documentation, which it builds from CLM's repository, is public by design, and it may name and link repositories, such as CLM's own in its install commands. Adding a login edits that list, and lands on checks like any change without a burden.

**Scenario: stranger signs in**
- GIVEN a GitHub login not on the list
- WHEN it signs in to the web app
- THEN it is refused and sees nothing

**Scenario: stranger opens the site**
- GIVEN someone with no session
- WHEN they open the homepage and the guide, and then ask the API for the repo list
- THEN they read the site, and the API answers 401
