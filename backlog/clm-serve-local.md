---
title: "clm serve: the web app on localhost over a local clone"
kind: idea
components: [replica, clm]
---

Run the same browser app on `localhost` with `replica`'s core over a local clone: a git source (refs from `.git`, changed files from `git diff --name-only old new`, text from the object store) and an in-memory or SQLite store, with no sign-in, no tokens and no CPU or rate limits. Live state (claims, spec branches) comes from local and `origin` refs; pull requests through `host` when it has an adapter. The core is host-neutral for this (`replica/host-neutral`, `web-repo-replica`). Deferred by the human on 2026-11-02 to keep `20261102-web-repo-replica` on the hosted app.
