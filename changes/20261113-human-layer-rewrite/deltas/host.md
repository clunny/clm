---
component: host
---

## MODIFIED Requirements

### Requirement: branch-push <!-- base: 2c314eee6b3834a0 -->
`clm branch push` SHALL push the checked-out branch to `origin`'s branch of the same name and set its upstream there, on every host: when the clone has `refs/remotes/origin/<branch>`, only while `origin`'s branch is still at that ref's tip and the local branch has held that tip, as git's `--force-if-includes` judges it; when it hasn't, only while `origin` has no such branch. It SHALL work on the branch checked out in the directory it runs in, and SHALL print `pushed <branch> <sha>`, `<sha>` the full commit `HEAD` names, and exit 0 when the push goes through or `origin` already holds that commit. With no `origin` remote it SHALL push nothing and exit 1. When git refuses the push it SHALL print git's message, the remote's lines included, and exit 1, leaving `origin`'s branch as it was. On a detached `HEAD`, on `main` or on a branch whose name starts with `clm/`, it SHALL push nothing and exit 1 naming the reason.

#### Scenario: rebased branch pushed
- GIVEN branch `impl/a` checked out, pushed to `origin` at `s` and fetched, then rewritten by a rebase to `t`
- WHEN `clm branch push` runs
- THEN `origin/impl/a` is `t`, the branch's upstream is `origin/impl/a`, it prints `pushed impl/a t` and exits 0

#### Scenario: new branch
- GIVEN branch `spec/x` checked out at `s`, with no `origin/spec/x` and no remote-tracking ref for it
- WHEN `clm branch push` runs
- THEN `origin/spec/x` is `s`, the branch's upstream is `origin/spec/x`, and it exits 0

#### Scenario: moved since the last fetch
- GIVEN branch `impl/a` checked out and rebased, and `origin/impl/a` pushed to `u` by another clone since this clone's last fetch
- WHEN `clm branch push` runs
- THEN `origin/impl/a` is still `u`, the output holds `stale info`, and it exits 1

#### Scenario: fetched but never held
- GIVEN branch `impl/a` checked out at `t`, and `origin/impl/a` pushed to `u` by another clone and fetched into `refs/remotes/origin/impl/a`, `u` never in the local branch's reflog
- WHEN `clm branch push` runs
- THEN `origin/impl/a` is still `u` and it exits 1

#### Scenario: main refused
- GIVEN `main` checked out with a commit `origin/main` lacks, and in turn a detached `HEAD`, and a branch `clm/ledger/p/c` checked out
- WHEN `clm branch push` runs in each
- THEN `origin` is unchanged and it exits 1 each time

#### Serves
- capability/git-host
- capability/build-from-spec
- capability/triage-unattended
- capability/work-concurrently
- architecture/land-locally
