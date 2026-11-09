---
title: "The after-land verify reads a stale checkout's settings and silently skips"
kind: bug
components: [verify, plugin]
---

At the end of /clm:build, the skill runs `clm verify --due --event work-done` in the session's own checkout, while the same skill forbids bringing that checkout up to date. The checkout was 40+ commits behind origin/main, and the [verify] section had landed on main during the session, so `clm verify --due` read the checkout's clm.toml, found no [verify] and printed `verify off: no [verify] in clm.toml`, exiting 0. The slow tests never ran, and nothing marked that as anything but normal. Run from a worktree at origin/main, the same command said a run was due (queue-drained) and ran it.

/clm:triage's last step runs the same command in its own checkout, so it can skip in the same way.

Expected: `clm verify --due` judges whether a run is due, and how to run it, from origin/main's [verify] settings (it already verifies origin/main's head), or the skills run it somewhere that is at origin/main. At the very least, a `verify off` read from a checkout that is behind origin/main should say so.

Rules involved: verify-after-land, views-name-their-commit, and the build skill's rule to leave the session's checkout as it is.
