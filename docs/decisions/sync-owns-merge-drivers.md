---
title: "clm sync configures its merge drivers per invocation"
date: 2026-10-22
status: accepted
---

Git merge drivers need two halves: an attribute mapping paths to a driver (normally a committed `.gitattributes`) and the driver's command (only in git config, which isn't committed, so normally set up once per clone). Rather than ask every CLM repo and every clone for that setup, `clm sync` passes both on the command line each time: `-c merge.<name>.driver=...` and `-c core.attributesFile=.git/clm/attributes`, a file it writes inside `.git`.

So the drivers apply exactly when CLM's rules apply, which is inside `clm sync`. A plain `git rebase` falls back to git's line merge, which is what it does today. When a rebase stops, `clm sync` resumes it with the same configuration, which is why callers run `clm sync` again rather than `git rebase --continue`.

GitHub's server-side merge runs no merge drivers at all, so a PR that conflicts on GitHub needs `clm sync` run from a session (`ci-sync-conflicting-prs`).
