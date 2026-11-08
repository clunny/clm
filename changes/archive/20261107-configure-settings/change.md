---
status: done
priority: P2
resolves: [settings-changed-only-by-hand]
---

## Intent

Give CLM's settings one place that says what each does and what each choice trades off, a command that lists them all with where each came from, and a `/clm:configure` skill that reviews every setting, or goes straight to one named in plain words, edits the right file and lands the edit the right way. Today the only way to change a setting is to edit `clm.toml` by hand, guided by a reference table that has already drifted from the code.

## Problem

After setup, changing how CLM behaves means editing `clm.toml`, `clm.local.toml` or the user's `config.toml` by hand. The guide's reference page has a settings table copied by hand from `config`'s Contract: it already lacks `verify` and the `azure` host, and it says nothing of trade-offs. `clm config get` shows one key at a time; nothing lists them all or checks an edit before it lands. And how an edit lands depends on the key and the file: `clm.local.toml` and the user's file are never committed, a `clm.toml` edit lands on checks, and an `approval` edit needs review.

## Approaches

1. **A settings guide page, `clm config list` and the skill** (chosen). `docs/guide/settings.md` describes every key and its trade-offs, with a test that fails when it leaves out a key `clm-config` accepts; `clm config list` prints every key, value and layer, and validates by failing; `/clm:configure` reads both, edits files in place and validates with `clm config list`. Knowledge sits in the guide, which every project's `clm` carries, so `/clm:help` and plain chat reach it too.
2. **Skill alone**, its knowledge in prose, calling `clm config get` per key. Cheapest, but nothing catches drift and nothing else reaches the knowledge.
3. **A full CLI**: `clm config list`, `set` and `explain`, descriptions in Rust. `set` must handle tables, layers that never merge a table, and comments: much code for edits the agent already makes well, which `list` validates (`yagni`).

The human chose 1 (`settings-guide-page`).

## Outcome

- `docs/guide/settings.md` (`clm guide settings`) describes every key, grouped as the project's process, sessions on this machine, thresholds and upgrades, and replaces the reference page's table, which now points at it. This branch writes it.
- `config`: `clm config list` (`config-list`) and a test that the page covers every key (`settings-guide-covers-keys`).
- `plugin`: `skills/configure/SKILL.md` (`configure-skill`); `propose` tells a change that adds a key to write its section on the spec branch (`propose-adds-settings-sections`); `help` points at `/clm:configure` for a question about a setting (`help-skill`).

With no argument, `/clm:configure` reviews every setting in the page's order, showing each one's value, where it came from and its trade-offs, and asks whether to change it. With an argument such as `set concurrent builds`, it finds the key (`max_builds`) and goes straight to it. It leaves `upgraded`, `declined_migrations` and `skip_migrations` to `/clm:upgrade`.

## Constraints

- A section may name a key the code doesn't parse yet, since the spec branch carrying it lands before the build; the coverage test fails only on a missing section.
- `clm config list` and `get` read one ordered list of keys, so they can't disagree.
- In this repository, a key still lands before `clm.toml` sets it (`settings-before-use`); the skill sets only keys the installed `clm` accepts, which `clm config list` checks.
