---
title: "clm verify takes its settings from the clm.toml committed at origin/main"
date: 2026-11-09
status: accepted
---

`clm verify` verifies `origin/main`'s head, but it read `[verify]` from the `clm.toml` of the checkout it ran in. `/clm:build` and `/clm:triage` run `clm verify --due --event work-done` at their end in the session's own checkout, which they leave as it is and which can be many lands behind `main`. When `[verify]` landed after that checkout was made, the call printed `verify off: no [verify] in clm.toml` and exited 0: the slow tests never ran, and the line looked normal. The reverse holds too: a checkout still holding a `[verify]` that `main` has since removed or changed would run the old command against `main`.

`clm verify` now fetches first and takes `[verify]` and `[affected]` from the `clm.toml` committed at `origin/main`, on every host (`none-keeps-its-own-remote`), through `config`'s `load_committed`, and its `verify off` line names that file, `origin/main:clm.toml`, so the line says which commit it judged (`views-name-their-commit`). The commit it checks out and the settings it follows are then the same commit's. The clone's `clm.local.toml` still overrides, as it does for every setting (`local-settings`): it belongs to the clone, is never behind, and a clone may run its slow tests differently. `host` comes from there too (amended 2026-11-22, below).

Rejected: having the skills run `clm verify` in a worktree at `origin/main`. Every caller, a host pipeline and a human at the terminal included, would have to know to do so, and the build skill's rule to leave the session's checkout alone would grow an exception; the command already knows which commit it judges. Also rejected: only warning when a `verify off` comes from a checkout behind `origin/main`. It names the stale read but still skips the run the project asked for, and leaves a checkout that has `[verify]` while `main` doesn't running a check `main` no longer wants.

The cost: a project with no `[verify]` now fetches once per `clm verify` call before saying `verify off`, and a fetch that fails is exit 2 where it used to be `verify off`. The skills go on to their wrap-up whatever the call exits, so that costs a line, not a session.

## Amended 2026-11-22: `host` from `main` too

`host` first stayed the root's, as `land-policy-from-base` keeps it for `clm land`, on the grounds that it was read before there was anything fetched and described the clone's tooling rather than what is verified. Neither held. The fetch needs no setting, and `verify` uses `host` only to read the runs of a pipeline that ran `main`'s commits, which `main`'s value names; a clone that reaches its host differently sets it in `clm.local.toml`, which still wins. Loading the root's settings for that one key cost the guarantee this decision made: the load parses the working tree's `clm.toml` whole, so a checkout whose `[verify]`, `[affected]` or `[env]` table its `clm` couldn't load (a table whose keys changed since the checkout was made, say) refused every `clm verify` with exit 1, naming a file whose `[verify]` it promised never to take. Every setting now comes from the one load of `origin/main:clm.toml`, and `clm verify` reads nothing from the working tree's `clm.toml`. The ledger's writer identity, read for every record `verify` appends, stops loading the repository's `clm.toml` too, since `writer` is never set there (`ledger/writer-identity`).

Rejected: a `config` call that loads `host` alone from the root's layers. It keeps a second load of settings `verify` has no use for, and a new call in `config` for one caller, where the load `verify` already makes holds the key.
