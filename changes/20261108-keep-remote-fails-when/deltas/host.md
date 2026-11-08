---
component: host
---

## MODIFIED Requirements

### Requirement: none-keeps-a-remote <!-- base: 85ff1b8b1599f42a -->
`clm-host` SHALL offer `keep_remote(root, host)`. When `host` is `none` and the clone has no remote named `clm-git`'s `REMOTE`, it SHALL make `clm/remote.git` under the clone's common git directory (`clm-git`'s `common_dir`) a bare repository, keeping one already there, add it as `REMOTE` by its absolute path, and return that path; it SHALL push nothing and write nothing in any worktree. On any other host, or when the clone has `REMOTE`, it SHALL change nothing and return nothing. Run from a linked worktree, it SHALL give every worktree of the clone the same `origin`. It SHALL make the bare repository through `clm-git`'s `git_isolated`, so a clone the environment names with `GIT_DIR` and `GIT_WORK_TREE` (a work tree whose git directory is elsewhere) gets its remote too, while reading the remotes, `common_dir` and adding `REMOTE` still run on the clone those variables name.

#### Needs
- vcs/git-isolated

#### Scenario: no remote
- GIVEN `host = "none"`, a clone with a commit on `main` and no remote
- WHEN `keep_remote` runs
- THEN `<common git dir>/clm/remote.git` is a bare repository, `git remote get-url origin` prints its absolute path, it has no branch, and the working tree is unchanged

#### Scenario: from a linked worktree
- GIVEN the same clone with no remote, and a linked worktree of it
- WHEN `keep_remote` runs in the linked worktree
- THEN the bare repository is under the common git directory, not the worktree's, and `git remote get-url origin` prints the same path in both checkouts

#### Scenario: a clone the environment names
- GIVEN `host = "none"`, a work tree `w` with a commit on `main` and no remote, whose git directory `g` is elsewhere, and `GIT_DIR=g` and `GIT_WORK_TREE=w` set in the environment `keep_remote` runs in
- WHEN `keep_remote(w, none)` runs
- THEN it succeeds, `g/clm/remote.git` is a bare repository, and `git remote get-url origin`, run with those variables, prints its absolute path

#### Scenario: already kept
- GIVEN the clone after *no remote*, its `origin` then removed with `git remote remove origin`
- WHEN `keep_remote` runs
- THEN `origin` names the same bare repository again, and what it held is still there

#### Scenario: nothing to do
- GIVEN `host = "none"` with an `origin` naming another bare repository, and in turn `host = "git"` with no remote
- WHEN `keep_remote` runs in each
- THEN the remotes are unchanged, no `clm/remote.git` exists, and it returns nothing
