---
component: sync
---

## MODIFIED Requirements

### Requirement: judged-by-check <!-- base: 5c30baa7e7a41917 -->
After a rebase that completes and any regeneration, `clm rebase` SHALL load the settings on every branch, printing the error and exiting 1 when they don't load, and SHALL run every `clm check` rule on the result, print any violations, and exit 1 if there are any, 0 otherwise. It SHALL load the settings and run the check on the branch's `HEAD` commit, checked out in a temporary worktree outside the repository, never on the working tree, so no uncommitted edit and no untracked or ignored file changes the outcome, and SHALL remove that worktree before it exits. A file that a settings error or a violation names SHALL be printed by its path in the repository, never by its path in the temporary worktree.

#### Scenario: merged into a broken state
- GIVEN a branch whose test cites `req:a/x`, and an `origin/main` that removed requirement `a/x`
- WHEN `clm rebase` runs
- THEN the rebase completes, it prints the dangling marker, and it exits 1

#### Scenario: settings can't be loaded
- GIVEN branch `spec/x`, and an `origin/main` whose `clm.toml` has a key the running `clm` doesn't know
- WHEN `clm rebase` runs
- THEN the branch is rebased, it prints a line naming `clm.toml` by its path in the repository (no part of it under the temporary directory) and the error, its last line is `rebase incomplete`, and it exits 1

#### Scenario: uncommitted settings edit
- GIVEN branch `spec/x` behind `origin/main`, with an uncommitted edit to `clm.toml` that adds a key the running `clm` doesn't know, which `origin/main` hasn't changed, and nothing else uncommitted
- WHEN `clm rebase` runs
- THEN the branch is rebased, it prints no line naming `clm.toml`, its last line is `rebase ok`, it exits 0, and `clm.toml` still holds the edit, uncommitted

#### Scenario: uncommitted marker
- GIVEN branch `spec/x` whose committed tree passes `clm check`, with an uncommitted edit to the tracked `src/lib.rs` that adds a test citing `req:a/missing`, a requirement no component has, which `origin/main` hasn't changed
- WHEN `clm rebase` runs
- THEN it prints no `check/dangling-markers` violation, its last line is `rebase ok`, and it exits 0

#### Scenario: committed fault, uncommitted fix
- GIVEN branch `spec/x` whose committed `src/lib.rs` cites `req:a/missing`, a requirement no component has, with an uncommitted edit that removes the citation, and an `origin/main` that hasn't changed `src/lib.rs`
- WHEN `clm rebase` runs
- THEN it prints a `check/dangling-markers` violation whose path is `src/lib.rs`, its last line is `rebase incomplete`, it exits 1, and `src/lib.rs` still holds the edit, uncommitted

#### Scenario: untracked test file
- GIVEN branch `spec/x` whose committed tree passes `clm check`, with an untracked, unignored `tests/new.rs` citing `req:a/missing`, and nothing else uncommitted
- WHEN `clm rebase` runs
- THEN it prints no violation, its last line is `rebase ok`, it exits 0, and `tests/new.rs` is still there, untracked

#### Scenario: no temporary worktree left
- GIVEN `TMPDIR` set to an empty directory, and branch `spec/x` whose committed tree has a violation
- WHEN `clm rebase` runs
- THEN it prints the violation and exits 1, `TMPDIR` is empty again, and `git worktree list` names only the repository's own worktrees

#### Scenario: a clone the environment names
- GIVEN branch `spec/x` up to date with `origin/main`, whose committed tree passes `clm check`, with an untracked, unignored `tests/new.rs` citing `req:a/missing`, in a work tree `w` whose git directory `g` is elsewhere, with `GIT_DIR=g` and `GIT_WORK_TREE=w` set in the environment `clm rebase` runs in
- WHEN `clm rebase` runs
- THEN it prints no violation, its last line is `rebase ok`, it exits 0, and, run with those variables, `git rev-parse HEAD` names the commit it named before and `git status --porcelain` prints only `?? tests/new.rs`

#### Serves
- capability/catch-drift
- architecture/no-silent-drift

#### Needs
- vcs/git-follows-its-clone
