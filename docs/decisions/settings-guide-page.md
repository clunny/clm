---
title: "Settings are described once, in the guide's settings page, and changed by a skill that edits and validates"
date: 2026-11-07
status: accepted
---

Changing CLM's settings meant editing `clm.toml` by hand. The only description was a table in the guide's reference page, copied by hand from `config`'s Contract, which had already drifted (it lacked `verify` and the `azure` host) and said nothing of trade-offs. So:

- **One page describes every setting**: `docs/guide/settings.md`, printed by `clm guide settings`, says for each key what it does, what each choice trades off and which files may set it. It replaces the reference page's table. Being a guide page, it reaches every project from its `clm` (`method-in-the-plugin`), so `/clm:help`, plain chat and `/clm:configure` read the same text.
- **A test keeps it complete** (`config/settings-guide-covers-keys`): every key `clm-config` accepts must have a section. It doesn't fail on a section for a key the code doesn't parse yet, since a change writes the section on its spec branch, which lands before the build that parses the key, as the glossary's entries do; `propose` says so (`plugin/propose-adds-settings-sections`).
- **`clm config list`** prints every key, its value and where it came from in one call, and fails naming the file and key when a value is wrong, so the skill holds no parser and validates an edit by running it.
- **`/clm:configure` edits files in place**, rather than through a `clm config set`: the agent already edits TOML well, keeping comments; tables such as `[land]` and `[[detectors]]`, and layers that never merge a table, would make a `set` command large for no gain the validation step doesn't give (`yagni`). It reviews every setting with no argument, or goes straight to one named in plain words, and lands a `clm.toml` edit on a branch of its own by `clm gate`'s first line, so an `approval` edit gets its review (`approval-setting-is-spec`).

Rejected:
- **Knowledge in the skill alone.** Nothing would catch drift, and plain chat and `/clm:help` couldn't reach it.
- **Descriptions and trade-offs in Rust, with `clm config set` and `explain`.** Prose in code, and the most code for the least gain.
- **A test that also fails on a section for an unknown key.** It would fail on `main` between a key's spec change and its build.
