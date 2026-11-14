---
title: "A test cites the version of the requirement it proves, and a PR merges with whatever it made current"
date: 2026-10-23
status: accepted
---

`proven-by-tests` made a requirement implemented when any test cited its ID. A modified requirement then passed coverage on its old test, and no program could tell that the code hadn't moved. The only record of what a build was built to was a task's bundle hash, tested by `gate/task-done-fresh` and `clm sync`'s `basis changed`: coarse (the whole bundle), per task rather than per requirement, and failing a finished PR whenever `main` moved under it, which used up the one resume a session was allowed.

Now a marker is `req:<node>/<slug>@<version>`, the version being the first 8 hex digits of the requirement's content hash (`model/content-hash`). The marker is the per-requirement record of what the code was built to:

- **current**: every marker that cites it cites the current version (`stale-markers-keep-behind`);
- **behind**: a marker cites another version, or none, so the text moved after the test was written, or the test predates versions;
- **unbuilt**: nothing cites it;
- **obsolete**: a marker cites a requirement a live change removed (`removed-requirements-build-out`).

`clm pending` lists the last three, and the queue is built on it (`node-reconciliation`). `dangling-markers` already treated markers as the record of what exists; this extends it to which text.

**Partial progress merges.** A PR's claim is exactly the versions its markers cite. A requirement whose text moved while the PR was open stays `behind` and is built again by the next run; the PR itself still merges, since everything it does cite is true. Nothing on `main` is silent (`no-silent-drift`): `clm status` and `clm pending` show what is behind. So no gate rule tests a build's freshness, and no resume is forced by a moving `main`.

**Sync at fixed points, without a basis.** A session still runs `clm sync` after each step of its plan and before its PR, never on a timer. On `impl/<node>` the sync compares the node's bundle between the branch's merge base and `origin/main`, two trees git has, and reports `spec changed` once, with the requirements whose versions moved, for the session to read and adapt. There is no recorded basis to refresh and no re-plan.

**Cost.** A wording change to a requirement makes its tests `behind` until a build confirms them and re-cites the new version. That is the point: someone checks that the test still proves the new text. The build sees the diff, so the check is cheap.

**Migration.** Existing markers carry no version, so they count as `behind`. The first code PR of the queue redesign stamps each with the current version, except a requirement MODIFIED by a live change whose code never caught up, which is stamped with the delta's recorded base so it stays behind. That one-off keeps the first runs from re-verifying every requirement in the repository.

Rejected:
- **A per-node status file listing requirement versions.** A second record of what the markers already say (`regenerate-dont-merge`), with a writer to coordinate.
- **Blocking a PR whose marker is behind.** It forces a resume for text that moved after the test was written, with no evidence the code is wrong; merging and leaving the requirement pending costs one targeted rebuild instead, and nothing is hidden.
- **A version counter instead of a hash prefix.** Counters need a writer; the hash follows from the text (`hash-based-staleness`).
