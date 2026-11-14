---
title: "A land checks code in one tree per clone, .clm/land-tree, held under the land lock"
date: 2026-10-28
status: accepted
---

Every `clm land` attempt that runs `[land] check` takes the lock on `<main>/.clm/land.lock` and checks the head and the merge commit out in `<main>/.clm/land-tree`, a detached worktree of the repository that stays from land to land, and holds the lock until the merge commit's push has gone through, or the attempt ends sooner. The places are under the `.clm/` of the CLM root in the main worktree, which every project ignores whatever its language (`land-places-ignored`). An attempt that runs no check (a merge that conflicts, no `[land] check` set, or checked code whose readers select nothing, `doc-readers-marked`) and `clm approve` use a fresh temporary worktree, removed on exit. Before each checkout the tree is brought to the commit with `git checkout --force --detach` and `git clean -fdx`; a missing or unregistered tree is removed, pruned and added again. This refines `build-places`, whose lock ended with the check.

`clm land` sets neither `CARGO_TARGET_DIR` nor `CARGO_INCREMENTAL`: it hands the check `CLM_LAND_DIR`, and the project's check script points its toolchain there (`land/shared-land-target`, `init/starter-check-land-dir`). So a caller's `CARGO_TARGET_DIR` exempts no land from the lock and the land tree. The rule is one: lock means land tree.

**Why.** `build-places` shared one compile directory so lands compile warm, but the tree each land compiled was a `land-*` worktree at a random path under `TMPDIR`, itself the session's own under Claude Code. Cargo keys what it builds by the workspace's path: the paths in the dep-info it judges freshness by, the `CARGO_MANIFEST_DIR` a test bakes in with `env!`, its incremental sessions. On 2026-10-27, after a day of lands, the shared land target held 43 GB (`debug/deps` 27 GB, `debug/incremental` 15 GB for checkouts already deleted), each land had compiled the whole workspace again, and one land ran `clm-affected`'s `git_layer` test from a binary cargo called fresh whose `CARGO_MANIFEST_DIR` named an earlier land's deleted tree, so it panicked on a missing `Cargo.toml`. With one path, unchanged crates are fresh and correctly so: the tree is the one the last land compiled, `git checkout` touches only the files that differ, and the lock keeps two lands from touching it at once; outputs overwrite instead of piling up; and `CARGO_MANIFEST_DIR` names a tree that exists.

**Why a fixed path in the main worktree.** `scratch-outside-git` moved the scratch tree out of `.git/`, where Vite refused it, into the temporary directory. A fixed path can't be there: `TMPDIR` changes per session. `<main>/.clm/` is ignored by the repository, holds the land's compiled code and its lock beside the tree, and has no `.git` component. A path in the git common directory would put `.git` back in the path; a sibling of the clone would write outside it. Removing `.clm/land` and `.clm/land-tree` clears the tree with the build it served.

**Why the lock lasts to the push.** The gate on the merge commit and `clm check` read the tree after `[land] check`; released at the check's end, as `build-places` had it, the lock would let the next land check its own commit out under this one's gate. Holding it through the push keeps the tree this land's until its commit is on `main`; the push's retries wait at most 3 seconds per attempt, nothing against a compile, and a land retrying after `main` moved releases the lock before it fetches. The `landed` append and the branch delete read no tree, so they run outside it.

**Why not every land in the tree.** A spec land that runs no test compiles nothing and would wait behind a code land's check; `build-places` chose the lock's narrow scope so it doesn't. The two kinds of tree cost one rule: lock means land tree.

**Why `clean -fdx`.** Whatever a check leaves behind, built or ignored, must not reach the next land's check, which judges another commit; `npm ci` removes `node_modules` itself, so the ignored files the clean costs are ones the check rebuilds anyway.

**Verdicts that don't rest on this tree.** Two, both accepted for the same reason: a check that passes only through an ignored file it doesn't make itself is the project check's defect, and `[verify]` or the next build's full check shows it.
- A land whose merged tree's code key a build's passing `clm land --check-only` recorded runs no check of its own (`worktree-check-counts`), so that verdict rests on the build's worktree, beside its own `target/` and other ignored files.
- A land whose branch's head was checked whole and whose fetched `main` is checked, and whose overlap selects no test, runs no check and appends a composed `checked` record (`land/overlap-recheck`). That verdict rests on the build's recorded check and on `main`'s.

**Alternatives.**
- *A target directory per scratch tree.* Correct but cold every time, and the directories pile up: the problem `build-places` solved.
- *Make cargo rebuild when the path changes* (`CARGO_INCREMENTAL=0`, touching sources). Never warm, and leaves the old paths' outputs in place.
- *Prune stale artifacts.* Cargo offers nothing between leaving them and `cargo clean`.
