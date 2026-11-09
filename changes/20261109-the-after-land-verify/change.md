---
status: applied
resolves: [the-after-land-verify-reads-a-stale-checkout-s-settings-and]
---

## Intent

Make `clm verify` judge `origin/main` by the settings committed at `origin/main`, not by the `clm.toml` of the checkout it happens to run in. `/clm:build` and `/clm:triage` run `clm verify --due --event work-done` at their end in the session's own checkout, which they leave as it is and which can be dozens of lands behind `main`; when `[verify]` had landed after that checkout was made, the call printed `verify off: no [verify] in clm.toml` and exited 0, so the slow tests silently never ran (`backlog/the-after-land-verify-reads-a-stale-checkout-s-settings-and`).

## Problem

`clm verify` already fetches and verifies `origin/main`'s head, checked out into its own verify tree, but it loads `[verify]` (and `[affected] all`, which shapes its test selection) from the working tree of the CLM root before it fetches. The command's subject and its settings then come from different commits. A stale checkout can skip a run `main` asks for, as reported, or run a check `main` has since changed or removed. The `verify off` line names no commit, so nothing showed the read was stale (`views-name-their-commit`).

## Approaches

- *Read the settings from `origin/main` (chosen).* `clm verify` loads `host` from the root, fetches `origin` (on every host: `none-keeps-its-own-remote`), then loads the rest through a new `config` call, `load_committed`, whose repository layer is the text of `origin/main:<prefix>clm.toml`; `clm.local.toml` still overrides, as for every setting. The `verify off` line names `origin/main:clm.toml`. Every caller (the skills, a host pipeline, a human) gets the right answer with nothing to know, and the commit verified and the settings followed are the same commit's. It mirrors `land-policy-from-base`, which reads `approval` from the base for the same reason.
- *Have the skills run `clm verify` in a worktree at `origin/main`.* Rejected: each caller would have to know it, and the build skill's rule to leave the session's checkout alone would need an exception, while the command already knows which commit it judges.
- *Only warn when `verify off` comes from a checkout behind `origin/main`.* Rejected: it names the stale read and still skips the run, and does nothing for a checkout holding a `[verify]` that `main` has changed.

`plugin` needs no change: its skills keep running `clm verify --due --event work-done` in their own checkout, which is now correct.

## Design

- `config` Contract and `config/committed-settings` (added): `clm_config::load_committed(root, text, source, env)` loads as `load` does with the repository layer taken from `text` (none for `None`), never from `<root>/clm.toml`, each value it sets and each error naming `source`.
- `verify/settings-from-main` (added): every `clm verify` takes `host` from the root, fetches, then takes `[verify]` and `[affected]` from `load_committed` over `origin/main:<prefix>clm.toml`; with no `[verify]` there it prints `verify off: no [verify] in origin/main:clm.toml`. `verify-due`'s off sentence and scenario follow, and the Contract and Design say where settings come from.
- `clm/verify-command` (scenario only): the dispatch scenario's `verify off` line names `origin/main:clm.toml`.
- Decision `verify-settings-from-main`; the guide's `settings` page says `[verify]` takes effect once it lands.

## Outcome

A session at any commit runs, or skips, the slow tests exactly as `origin/main`'s `clm.toml` (and the clone's `clm.local.toml`) say, and a `verify off` line says which commit's file it read.

## Edge cases and workarounds

- A project with no `[verify]` now fetches once per `clm verify` call before printing `verify off`; offline, that call exits 2 instead. The skills go on to their wrap-up whatever it exits.
- A `clm.toml` on `origin/main` that the running `clm` can't parse (a key newer than the binary) fails with exit 1 naming `origin/main:clm.toml`, where before the checkout's older file loaded.
- The first load, for `host`, still loads the checkout's whole settings, so a checkout whose `clm.toml` doesn't load still fails, naming that file, as today.
- `host` itself is still read from the checkout, so a host change on `main` reaches `clm verify` only once the checkout has it; it names the tooling, as `land-policy-from-base` keeps it for `clm land`.

## Constraints

- `views-name-their-commit`: the `verify off` line names the file and commit it read.
- `git-is-truth`: the settings are read from git, at the commit verified.
- `local-settings`: `clm.local.toml` keeps overriding, as everywhere.
- `reuse-first`: `load_committed` reuses `config`'s loading and validation, rather than a second parser for one table as `clm-approval` keeps for `approval`.

## Open questions

None. The architecture audit found two things, both fixed here: `config`'s Contract now names `load_committed`, so `verify`'s build sees it in its context (`ancestor-scoped-context`); and the settings are read from `origin/main` on every host, `host = "none"` included, rather than from the local `main` there, as `none-keeps-its-own-remote` requires (the current code's `false => "main"` base for `none` is that decision's leftover, which the build of `settings-from-main` removes along with it).
