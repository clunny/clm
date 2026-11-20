---
component: sync
---

## MODIFIED Requirements

### Requirement: regenerate-renames <!-- base: 0a8137636a52041e -->
On a branch that adds exactly one change, live or new archived, whose `change.md` at `HEAD` sets `renames`, `clm rebase` SHALL, after step 1's refusals and the fetch and in place of the rebase and the re-apply, first ask `rename/replay` whether the branch replays against its merge base with `origin/main`. When it doesn't, it SHALL print `not a rename branch <path>` for each path whose content differs from the replay, change nothing, and exit 1. When it does, it SHALL make the branch one commit on `origin/main`, `Rename <change-id>`, holding what `clm rename <change-id>` produces from `origin/main`'s tree with that `change.md` placed live with status `proposed`, plus the files under `backlog/` that the branch added; when `clm rename` refuses, it SHALL print the refusal, leave the branch as it was, and exit 1. It then judges the new `HEAD` as for any branch (`judged-by-check`). It SHALL NOT push. It SHALL produce each such tree, the one it compares to name the differing paths included, in a temporary directory that it makes no git repository and in which it runs no git command of its own, as `rename/replay` produces the tree it checks, so `clm rename` reads it as the replay does, and a `GIT_DIR` or `GIT_WORK_TREE` the environment sets, naming the clone, never makes the clone's git directory reinitialized or its index staged in place of that directory.

It SHALL read each revision as the CLM root's tree, as `rename/replay` reads one (`fingerprint-covers-the-clm-root`): the entries under the CLM root, the directory it runs in (`root-from-cwd`), each path relative to it, wherever that root sits in its git repository, so the change it finds, the `change.md` it reads, the `backlog/` files it keeps and the paths it compares and writes are the root's. It SHALL run `clm rename` in the root's directory of each tree it extracts. Outside the root the replay holds the merge base's entries, so each path outside the root where `HEAD`'s entry differs from the merge base's, by path, mode or object ID, SHALL be printed as a differing path, named relative to the root, as the shortest path from the root to it, climbing out with `../`, as the gate names one (`gate/paths-under-the-root`). The commit it makes SHALL hold `origin/main`'s entries outside the root, and each path outside the root where that commit differs from the branch's `HEAD` SHALL be left in the working tree as the commit holds it; no other path outside the root is written.

#### Scenario: main moved under a rename
- GIVEN a branch holding the committed result of `clm rename r`, renaming `a/x` to `a/y`, and an `origin/main` that has since added a test citing `a/x` at its current version
- WHEN `clm rebase` runs
- THEN the branch is one commit on `origin/main`, the new test cites `a/y`, its last line is `rebase ok`, and it exits 0

#### Scenario: a clone the environment names
- GIVEN the branch and `origin/main` of *main moved under a rename*, in a work tree `w` whose git directory `g` is elsewhere, with `GIT_DIR=g` and `GIT_WORK_TREE=w` set in the environment `clm rebase` runs in
- WHEN `clm rebase` runs
- THEN it ends as in *main moved under a rename*, `git status --porcelain`, run with those variables, prints nothing, and `g/config` is byte for byte as it was

#### Scenario: a hand edit on a rename branch
- GIVEN a branch holding the committed result of `clm rename r` and a commit editing `crates/a/src/lib.rs`
- WHEN `clm rebase` runs
- THEN it prints `not a rename branch crates/a/src/lib.rs`, the branch is unchanged, and it exits 1

#### Scenario: a rename in a subdirectory root
- GIVEN a git repository whose CLM root is its subdirectory `tool/`, a branch holding the committed result of `clm rename r` run in `tool/`, renaming `a/x` to `a/y`, and an `origin/main` that has since added a test under `tool/` citing `a/x` at its current version and edited `README.md` at the repository's top
- WHEN `clm rebase` runs in `tool/`
- THEN the branch is one commit on `origin/main`, the new test cites `a/y`, `README.md` holds `origin/main`'s text, `git status --porcelain` run in `tool/` prints nothing, its last line is `rebase ok`, and it exits 0

#### Scenario: a file outside a subdirectory root
- GIVEN the branch of *a rename in a subdirectory root* with one more commit editing `README.md` at the repository's top
- WHEN `clm rebase` runs in `tool/`
- THEN it prints `not a rename branch ../README.md`, the branch is unchanged, and it exits 1

#### Needs
- rename/replay
- rename/finish
