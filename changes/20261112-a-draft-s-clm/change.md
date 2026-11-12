---
status: applied
resolves: [a-draft-s-clm-setup-is-denied-by-claude-code-s-auto-mode, draft-prompt-doesn-t-ask-about-promised-code-work]
---

## Intent

Two gaps in what unattended drafts are allowed and told to do.

`clm setup` joins the `clm` subcommands whose allow rules `clm init` seeds and `clm init --check-permissions` checks for (`init/init-seeds-permissions`), so Claude Code's auto mode no longer denies a draft's or build's `clm setup` and the draft can fix a code-only item instead of leaving it as `setup failed`. And the `clm draft` prompt records the code work a design-only change promises, and its builder check asks about it, as `/clm:propose` already does (`orchestrator.triage/draft-records-promised-code-work`, mirroring `plugin.propose/propose-archives-design-only`).

## Problem

**Setup denied.** `20261109-a-build-s-tests` had the `clm run` and `clm draft` prompts run `clm setup` before their first test run on a code branch (`tests-run-in-their-own-tree`), and a session that can't run it leaves its item (`draft-prompt-setup`). `20261105-unattended-steps-denied-in` made every unattended step that writes a `clm` command and had `clm init` seed allow rules for them, since a project's allow rule resolves before the auto-mode classifier. But `clm setup` was never added to that list (`UNATTENDED_RULES` in `crates/clm-init/src/init.rs`, and this repository's `.claude/settings.json`), so the classifier judges it, and it denied a draft's `clm setup` as "Unauthorized Persistence" (`20261112-platform-fake-dead-helpers`): the project's `[build] setup` runs `npm ci` in the worktree. No requirement states that `clm setup` is allowed, so this is a spec change (`code-fix-needs-a-written-rule`).

**Promised code work.** `20261112-propose-skill-doesn-t` had `/clm:propose` record the code work a change with no requirement edits relies on, and had its builder check ask about it. The draft prompt, the other path that writes spec changes, says neither, so a design-only draft can still promise code that no build does.

## Approaches

For setup:

1. **Add `setup` to the seeded rules** (chosen). One list feeds both the first-run seeding and `clm init --check-permissions`, which `/clm:build` and `/clm:triage` already run before their first launch (`plugin/unattended-permissions-check`), so every repository, set up before or after, is told to add the two rules before a draft or build needs them. This is the item's first suggestion, and it reuses the mechanism built for the same failure (`reuse-first`). The command `clm setup` runs is the project's own, committed in `clm.toml` as `[land] check` is, which the already allowed `clm land` runs, so allowing it trusts nothing new.
2. **Have the triage session ask the human for the permission before launching drafts.** That is what the permissions check already does once the rule is in the list; a separate ask would duplicate it. Rejected.
3. **Skip setup for a component whose tests need no npm package.** CLM names no toolchain (`tests-run-in-their-own-tree`): which component needs which packages is the project's knowledge, and the project's `[build] setup` can skip what it doesn't need. Deciding it in CLM would need a new per-component setting for one denied command that the allow rule already fixes (`yagni`). Rejected.

For promised code work:

1. **Say in the draft prompt what `propose-archives-design-only` says** (chosen): the same two outcomes (a requirement, or a `kind: deferred` item naming the code) and the same builder-check question, in the design paragraph and the builder-check paragraph.
2. **Point the draft prompt at the propose skill.** The draft runs alone and reads its prompt, not the skill; the rule must be in the text it follows. Rejected.

The draft doesn't archive a design-only change, as `/clm:propose` does: it lands it `applied`, and `clm rebase` archives it once complete (`archive-on-sync`). Recording the work is independent of that, so the requirement says "before it applies the change" rather than before archiving.

## Outcome

- `init/init-seeds-permissions` names `setup` between `rebase` and `verify`: twenty-four rules. Its build adds `Bash(clm setup)` and `Bash(clm setup *)` to `UNATTENDED_RULES` and to this repository's own `.claude/settings.json`, which `clm/session-commands` keeps equal to the list. `init`'s Design names the full list (it lacked `verify` too) and why `setup` is safe to allow.
- `orchestrator.triage/draft-records-promised-code-work` (new) has the `clm draft` prompt record promised code work and ask the builder check about it.

## Constraints

- No edge cases or workarounds are known.
- A repository set up before this change gets the two rules only when its human adds them; `clm init --check-permissions` names them before the next draft or build.
