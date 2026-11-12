---
component: verify
base: { contract: "fe9d71eef5920c5b" }
---

## MODIFIED Contract
Running a project's slow tests after the land, at the moments the project chooses, and recording the result in the ledger (`verify-after-land`). Every `clm verify` takes its `[verify]` and `[affected]` settings, and the `[land] target_cap_gb` that caps its build place, from the `clm.toml` committed at `origin/main`, which it fetches first, never from the checkout it runs in, so a checkout behind `main` judges `main` by `main`'s settings (`settings-from-main`). A project whose `origin/main` sets no `[verify]` (`config`) keeps its slow tests in `[land] check`, and every `clm verify` then prints `verify off: no [verify] in origin/main:clm.toml` and exits 0. Crate `clm-verify` offers, dispatched by `crates/clm`:
- `clm verify --due [--event work-done] [--dry-run]`: whether a trigger has fired since the last verified commit of `origin/main` (`verify-due`), and when one has, the verification of `origin/main`'s head: in `local` and `pipeline` mode by running `[verify] check` (`verify-run`), in `import` mode by reading the pipeline's result (`verify-import`). With `--dry-run` it only says whether a run is due.
- `clm verify --commit <sha>`: the verification of one commit of `main`, for a release or deploy step (`verify-commit`).
- `clm verify --import`: in `import` mode, the record of every completed pipeline run on `main` newer than the last record, run nothing; in the other modes it prints `verify nothing to import` and exits 0.
- `clm verify --bisect <sha>`: the land that broke a red commit, found by running `[verify] check` on the lands it covers (`verify-bisect`).

Every outcome is one line on stdout starting `verify `: `verify not-due <why>`, `verify due <trigger>`, `verify passed <sha> <n> lands`, `verify failed <sha> culprit <land|unknown> components <component>...`, `verify pending <sha>` and `verify nothing to import`, `<sha>` abbreviated as `git rev-parse --short` abbreviates it. It exits 0 when it passed, nothing was due or there was nothing to import; 1 when the verification failed, or with the reason when it can't run (a `<sha>` not on `origin/main`'s first parents, a `[verify]` it can't use); 2 when git fails; 3 when, with `--dry-run`, nothing is due; and 4 when the result is pending (`import`). Each verification it decides appends one `verified` record (`ledger`); nothing else writes one.

As a library call it offers `kept_log(root, commit)`: the path of the log a failed run of `commit` kept in this clone, or none (`kept-log`), so `orchestrator.triage`'s `clm intake` can point a session at a red result's output without knowing where verify keeps it (`path-ownership`).

## ADDED Requirements

### Requirement: kept-log
`clm-verify` SHALL offer `kept_log(root, commit)`, `root` a CLM root in any worktree of a clone and `commit` a full commit ID, returning the absolute path of the file in `.clm/verify-logs/` in the CLM root of the clone's main worktree, the directory `verify-run` keeps a failed run's log in, whose name is `<name>.log` with `<name>` a prefix of `commit` at least 7 characters long, the longest such name when several match, and none when no file matches or the directory is missing. It SHALL read no settings and fetch nothing: it reads only that directory, running no git command other than those that find the main worktree and the CLM root's place in it, and SHALL neither create nor remove a file. Matching by prefix rather than by the abbreviation `git rev-parse --short` gives now keeps a log found after the repository's abbreviations have grown longer than when the run named it.

#### Scenario: a red run's log
- GIVEN a clone with a linked worktree, `mode = "local"`, one land since a passing record and a `[verify] check` that exits 1, and `clm verify --due` run in the main worktree with a trigger holding
- WHEN `kept_log` is called with the linked worktree's CLM root and the head's full commit ID
- THEN it returns the path of the log `clm verify` named, which exists

#### Scenario: nothing kept
- GIVEN the same clone, and a `[verify] check` that exits 0
- WHEN `clm verify --due` runs and `kept_log` is called with the head's full commit ID
- THEN it returns none

#### Scenario: a longer name
- GIVEN `.clm/verify-logs/` holding `<first 7 of commit>.log` and `<first 9 of commit>.log`
- WHEN `kept_log` is called with `commit`
- THEN it returns the path of `<first 9 of commit>.log`
