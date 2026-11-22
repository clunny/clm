---
status: done
resolves: [clm-verify-refuses-on-an-invalid-table-in-the-checkout-s-own-clm]
---

## Intent

`clm verify` loads no settings from the working tree's `clm.toml`: it takes `host` from the `clm.toml` committed at `origin/main`, as it already takes every other setting it follows, and the ledger's writer identity, which every record it appends carries, stops loading the repository's `clm.toml`, which never sets `writer`. So a checkout whose own `[verify]`, `[affected]` or `[env]` table can't be loaded no longer stops `clm verify`, as `verify/settings-from-main` promised.

## Problem

`verify/settings-from-main` says `clm verify` SHALL never take `[verify]` or `[affected]` from the working tree's `clm.toml`, so that the sessions running `clm verify --due --event work-done` in a checkout far behind `main` judge `main` by `main`'s settings. But `Ctx::open` (`crates/clm-verify/src/lib.rs`) loads the root's settings with `clm_config::load(Some(root), env)` to read `host` alone, and that load parses the working tree's `clm.toml` whole. A checkout whose `[verify]` its `clm` can't load (no `check`, or a key changed since the checkout was made) makes every `clm verify` refuse with exit 1, naming the checkout's file. Found by the spec-auditor in `verify`'s wave-3 build.

Fixing that load alone isn't enough: every record `clm verify` appends goes through `ledger`'s `writer(root, env)`, which also calls `clm_config::load(Some(root), env)`, only to read `writer`, which `config/writer-setting` refuses in the repository's `clm.toml`. With `host` fixed, a run would check for many minutes and then fail at the append, exit 2, losing its record.

## Approaches

1. **`host` from `origin/main` too** (chosen). `verify` loads its settings once, after the fetch, through `config/committed-settings`, and takes `host` from them. The reasons the decision `verify-settings-from-main` gave for keeping `host` the root's don't hold: the fetch needs no setting, and `verify` uses `host` only to read the runs of a pipeline that ran `main`'s commits, which `main`'s value names; a clone that reaches its host differently sets `host` in its `clm.local.toml`, which `committed-settings` still reads. It removes a load instead of adding a call (`reuse-first`, `yagni`).
2. **A `config` call that loads `host` alone from the root's layers**, or a load that goes on past a table it won't use, as the item suggested. Rejected: it adds a call to `config` for one caller and keeps a second load of settings `verify` has no use for, where the load it already makes holds the key.
3. **Accept the refusal.** Rejected: the refusal comes from a file the requirement says `verify` doesn't follow, and the case the decision was made for (a session's checkout far behind `main`) is the one where the checkout's `clm.toml` is likeliest to be one the running `clm` can't load.

For the writer, `ledger`'s `writer` loads the settings through `clm_config::load_committed(root, None, ...)`, which reads every layer `load` reads but the repository's `clm.toml` (`config/committed-settings`, no text): exactly the layers `writer` can come from. No `config` change is needed.

## Outcome

- `verify/settings-from-main` (modified): `host` joins the settings taken from `origin/main:clm.toml`; `clm verify` loads no settings from the working tree's `clm.toml` (the filing land aside), so neither a value it sets nor a table in it that can't be loaded changes what `verify` does. Two scenarios are added: a checkout whose own `clm.toml` can't be loaded still has its slow tests run and recorded, and `host` is `main`'s, not the checkout's. It needs `ledger/writer-identity`.
- `ledger/writer-identity` (modified): `writer` reads `writer` from `CLM_WRITER`, `clm.local.toml` and the user's `config.toml` only, never loading the repository's `clm.toml`; a scenario shows a `clm.toml` that can't be loaded failing neither `writer` nor `append`.
- `verify`'s Contract and Design say so, the decision `verify-settings-from-main` is amended, and the guide's settings page says where `clm verify` takes its settings from (`method-in-the-plugin`, a gap the audit found that predates this change).

The test `crates/clm-verify/tests/import.rs::the_host_comes_from_the_checkout` proves the behaviour this change reverses: `verify`'s build rewrites it in place to the scenario "the host from main" (renaming it to say so is not a deletion). Nothing else is removed, so `deleted-tests.txt` is not needed.

## Edge cases and workarounds

- `config/writer-setting` still refuses `writer` in a repository's `clm.toml` wherever `load` reads that file (`clm config get`, `clm check`, every other command); only `ledger`'s `writer` no longer checks it, since it no longer reads the file.
- The `clm land` that files unproven requirements (`files-unproven`) still reads the root's settings, as every land does (`land-policy-from-base`): with a checkout whose `clm.toml` can't be loaded it prints `unproven: not filed: <why>` after the record is made, and a later call files the items. Moving that land onto `main`'s settings would change `land`, for a step that already recovers. The `clm` binary's checks before any command (`clm/upgrade-blocks-commands`, `clm/none-remote-before-commands`) read the checkout's file too; they are the `clm` component's and tolerate a file that won't load as they always have.
- Every other `clm` command still loads the working tree's `clm.toml` whole, as `config` says; only `clm verify`, whose requirement promises it follows `main`, stops reading it.

## What the requirements no longer state

- `verify/settings-from-main`: "Every `clm verify` SHALL take `host` from the settings of the CLM root it runs in". `host` now comes from the `clm.toml` committed at `origin/main`, with the clone's `clm.local.toml` still over it.

## Constraints

- `path-ownership`: `ledger`'s build changes `crates/clm-ledger/src/writer.rs`; `verify`'s build changes `Ctx::open` and its tests, after `ledger/writer-identity` is current (its `#### Needs`).

## Open questions

None.
