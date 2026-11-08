---
component: clm
---

## ADDED Requirements

### Requirement: none-remote-before-commands
Before running any command except `--version`, `help`, `guide` and `hook`, in a CLM repository found as for `upgrade-blocks-commands` (the command's `--root`, else `root-from-cwd`) whose `host` setting is `none`, and after `require_current` let the command run where it is called, `clm` SHALL call `clm-host`'s `keep_remote` for it. When that creates the remote, `clm` SHALL print `origin: <path> (the clone's own remote, host = "none")` on stderr and, when the clone has a local `main`, a second line saying to run `clm reconnect` to push `main` and the ledgers to it; it SHALL push nothing itself, so `main` moves only through `clm land` and `clm reconnect` (`land-locally`, `guard-blocks-push-to-main`). When `host` is `none` and `origin` names anything other than the clone's own remote, it SHALL print on stderr a warning that lands will push `main` to that `origin` and that a repository with a remote should set `host = "git"`. A failure of `keep_remote` SHALL be printed on stderr as a warning, and the command SHALL run all the same; two commands racing to add the remote leave one `origin`, the loser warning. Outside a CLM repository, or when the settings don't load, it SHALL do none of this.

#### Scenario: a repository set up before
- GIVEN a CLM repository with `host = "none"`, no remote, and a local `main` holding a land made with no remote and its ledger branch
- WHEN `clm next` runs, then `clm reconnect`
- THEN after `clm next` `origin` is the clone's own remote with no branch, and stderr names the remote and `clm reconnect`; after `clm reconnect` `origin/main` is the local `main`, the ledger branch is on `origin`, and it printed `reconnected 1 lands`

#### Scenario: the loop runs with no host
- GIVEN that repository after `clm reconnect`, and a branch `impl/a` cut from `origin/main`, with a commit, in a linked worktree
- WHEN `clm rebase`, `clm branch push` and `clm land impl/a` run in turn
- THEN `clm rebase`'s last line is `rebase ok`, `clm branch push` prints `pushed impl/a <sha>`, and `clm land` prints `landed <sha> code` with `origin/main` at that commit

#### Scenario: a real origin under none
- GIVEN a CLM repository with `host = "none"` whose `origin` is another bare repository
- WHEN `clm next` runs
- THEN the remotes are unchanged, and stderr warns naming `host = "git"`

#### Scenario: hooks and guide create nothing
- GIVEN a CLM repository with `host = "none"` and no remote
- WHEN `clm hook` and `clm guide quickstart` run
- THEN the clone still has no remote

#### Needs
- host/none-keeps-a-remote
- land/offline-land
