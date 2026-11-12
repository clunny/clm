---
status: done
resolves: [the-agents-md-upgrade-migration-breaks-clm-s-own-requirement-clm]
---

## Intent

Let CLM's own repository take the advised migration `20261112-agents-md` (`upgrade/migration-agents-md`) and keep every requirement true. Two rules about this repository's own instructions name `CLAUDE.md` as the file that holds them; after the migration the text lives in `AGENTS.md` and `CLAUDE.md` is the one line `@AGENTS.md`. This change makes both rules read the instructions wherever they live: `CLAUDE.md` and, when it exists, the `AGENTS.md` it imports, so they hold before the migration (as the repository stands today) and after it.

## Problem

`clm/claude-md-pushes-with-clm` says this repository's `CLAUDE.md` SHALL name `clm branch push`, and its test (`crates/clm/tests/claude_md.rs`) reads only `CLAUDE.md`. Once the migration moves the text, the test fails ("CLAUDE.md lacks clm branch push") and the upgrade branch can't land, since its land selects that test through its `clm:reads CLAUDE.md` marker. The fix changes the requirement's text, so it needs this spec change first (`code-fix-needs-a-written-rule`).

The same assumption, checked across every test marked `clm:reads CLAUDE.md`:

- `plugin.build/skill-named-build` forbids `/clm:implement` in `CLAUDE.md` and `README.md`. After the migration its test still passes, but silently stops checking the instructions, which have moved to `AGENTS.md`. The rule names `AGENTS.md` too.
- `crates/clm/tests/retired_terms.rs` (`clm/retired-terms`) already reads and marks `AGENTS.md`.
- `crates/clm/tests/plugin.rs` carries `clm:reads CLAUDE.md` but reads no root `CLAUDE.md`: its one `CLAUDE.md` is a needle in the contribution skill's text. The marker only makes a land run it more often; nothing changes.
- `clm/doc-readers-marked` already covers `AGENTS.md` through `ledger`'s `doc_only`, which names it.

`upgrade` needs no change: the migration does what `migration-agents-md` says. The gap was in this repository's own rules, which the change that added the migration (`20261112-any-harness`) didn't update.

## Approaches

1. **Read both files** (chosen). The rule covers `CLAUDE.md` and, when present, `AGENTS.md`: neither holds `git push`, and together they name `clm branch push`. It holds before and after the migration, so the rule can land before the upgrade branch and the upgrade branch lands on it unchanged.
2. **Name `AGENTS.md` only.** Matches the end state, but is false until the migration is taken here, so the upgrade and this change would have to land together on one branch, mixing a spec change and a migration. Rejected.
3. **Follow `CLAUDE.md`'s `@` imports generally.** Covers any import layout, but no project here imports anything but `AGENTS.md` (`yagni`). Rejected.
4. **Accept the failure and drop the requirement.** The rule guards `unattended-steps-run-clm` for this repository's sessions; the move doesn't make it less needed. Rejected.

## Outcome

- `clm/claude-md-pushes-with-clm` (modified): reads `CLAUDE.md` and, when it exists, `AGENTS.md`; neither names `git push`, and together they name `clm branch push`; its test marks `clm:reads AGENTS.md` beside `clm:reads CLAUDE.md`. The ID stays, since `CLAUDE.md` stays the file a Claude Code session loads.
- `plugin.build/skill-named-build` (modified): `AGENTS.md`, when it exists, joins the files that may not name `/clm:implement`; its test reads it and marks `clm:reads AGENTS.md`.

## Constraints

- `upgrades-carry-migrations`, `dogfood-from-main`, `doc-readers-marked`, `code-fix-needs-a-written-rule`, `yagni`.
- No edge cases or workarounds known.
