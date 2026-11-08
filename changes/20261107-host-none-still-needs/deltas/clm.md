---
component: clm
---

## ADDED Requirements

### Requirement: none-remote-before-commands
Before running any command except `--version`, `help`, `guide` and `hook`, in a CLM repository found as for `upgrade-blocks-commands` (the command's `--root`, else `root-from-cwd`) whose `host` setting is `none`, `clm` SHALL call `clm-host`'s `keep_remote` for it. When that creates the remote, `clm` SHALL print `origin: <path> (the clone's own remote, host = "none")` on stderr and, when the clone has a local `main`, SHALL run `clm-land`'s `reconnect` there and print its lines on stderr, so the remote gets `main` and the ledgers through `land` (`land-locally`). A failure of either SHALL be printed on stderr as a warning, and the command SHALL run all the same. Outside a CLM repository, or when the settings don't load, it SHALL call neither.

#### Scenario: a repository set up before
- GIVEN a CLM repository with `host = "none"`, no remote, and a local `main` holding a land made with no remote and its ledger branch
- WHEN `clm next` runs
- THEN `origin` is the clone's own remote, `origin/main` is the local `main`, the ledger branch is on `origin`, stderr names the remote and `reconnected 1 lands`, and `clm next` exits 0

#### Scenario: the loop runs with no host
- GIVEN that repository after `clm next`, and a branch `impl/a` cut from `origin/main`, with a commit, in a linked worktree
- WHEN `clm rebase`, `clm branch push` and `clm land impl/a` run in turn
- THEN `clm rebase`'s last line is `rebase ok`, `clm branch push` prints `pushed impl/a <sha>`, and `clm land` prints `landed <sha> code` with `origin/main` at that commit

#### Scenario: hooks and guide create nothing
- GIVEN a CLM repository with `host = "none"` and no remote
- WHEN `clm hook` and `clm guide quickstart` run
- THEN the clone still has no remote

#### Needs
- host/none-keeps-a-remote
- land/offline-land
