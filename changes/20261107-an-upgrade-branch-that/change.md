---
status: applied
resolves: [an-upgrade-branch-that-only-stamps-clm-toml-s-upgraded-key-runs]
---

## Intent

Let a land whose only code difference from checked code is `clm.toml`'s upgrade bookkeeping (`upgraded`, `declined_migrations`) or its comments count as checked, as a spec-only land does. Today such a land runs the whole `[land] check`: in a project whose check takes 15 minutes, a `/clm:upgrade` branch that took one note migration waited the full run, though no code, test or setting changed (GitHub issue #262). The `[verify]` migration (`20261107-slow-tests-after-land`) appends a commented section to `clm.toml`, so every project that takes it would pay the same again.

## Problem

`ledger`'s code key hashes the tree with the doc-only paths left out, and `clm.toml` is code: its settings, `[land] check` first, decide what a check runs. But `clm.toml` also holds the upgrade stamp, which no check reads, and comments, which no TOML reader sees. The gate already treats a stamp-only `clm.toml` as neutral (`gate/upgrade-stamp-left-out`); the land's check lookup doesn't.

## Approaches

- *Leave clm.toml's bookkeeping out of `code_key` itself* (hash the parsed settings without the stamp in place of the file's blob). One key everywhere, but every reader of code keys would need `clm.toml`'s text, not only the tree listing: `web.worker`'s replica and review compute code keys from the Trees API listing alone, and would spend a blob fetch per key against a 50-subrequest budget, or disagree with the CLI. Every project's checked keys would also go stale once. Rejected.
- *Move the stamp out of `clm.toml`* into a doc-only file. A breaking change across `config`, `upgrade`, `init`, `gate` and the plugin, with a migration of its own, and it does nothing for comments. Rejected: `upgrade-migrations` chose the stamp's place, and the gate's rule already makes its edit neutral.
- *Accept it.* An upgrade is rare, but each one costs a full check in every project, and the `[verify]` migration makes the comment-only case common. Rejected.
- *Chosen: a second lookup in `clm land`.* When the merged tree's key has no `checked` record and `clm.toml`'s settings are unchanged from the base (`gate`'s new `settings_unchanged`, the comparison `upgrade_stamp_only` already makes, without its "something differs" half), the land looks up the key of the merged tree with `clm.toml` as the base has it (`ledger`'s new `code_key_with`). A hit means the code was checked; the land goes on as a checked one, so the readers of `clm.toml` (`clm:reads clm.toml`) still run, and it appends a `checked` record for the merged tree's own key, so the `landed` record names a checked key for `clm ledger verify` and compaction, and a retry finds it directly. Nothing else reading code keys changes.

## Outcome

A `/clm:upgrade` branch that takes notes, and a migration that only adds comments to `clm.toml`, land in the time `clm check` takes.

## Edge cases and workarounds

- The Worker's "checked" chip for an upgrade branch's head still shows unchecked until it lands, since the Worker computes the plain code key. Accepted: the chip advises, and the land decides.
- The appended `checked` record is written for a tree whose `[land] check` didn't run in full. It is honest in what it states, that the tree's code and settings passed: the code is the checked code, the settings are the same, and the tests that read `clm.toml` ran.

## Constraints

- `deterministic-enforcement`: the lookup is a program's comparison of parsed TOML and tree listings.
- `reuse-first`: `gate`'s TOML comparison and `ledger`'s key are reused, not copied.
- `doc-readers-marked`: a test that reads `clm.toml` still runs through the readers' selection.
