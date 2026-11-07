---
status: applied
resolves:
  - triage-skill-doesn-t-say-what-to-do-with-clm-intake-s-issue
---

# Triage skill names `clm intake`'s imported lines

## Intent

Have the triage skill say what `clm intake`'s `issue <n> imported <slug>...` and `alert <n> imported <slug>...` lines mean and that they ask nothing of the human, and leave them out of the *Contributions* count on the menu. Today `orchestrator.triage/intake` prints these lines, but `plugin/triage-contributions` and `plugin/triage-menu` name every other line form and not these, so a session that met one had to read the spec to learn it means "already taken in, nothing to do" (`method-in-the-plugin`).

## Problem

`clm intake` prints `imported` for an open issue or alert that an item on `origin/main`'s backlog already names in its `issues` or `alerts`. The skill's step 1 lists the line forms but skips `imported`, step 2a gives it no handling, and the menu's *Contributions* count doesn't say whether it counts. A menu that counted them would offer a *Contributions* choice with nothing to do when every open issue is already imported.

## Approaches

- *Count them and list them under Contributions.* Rejected: a choice the human picks only to be told there is nothing to do costs a question for no decision.
- *Leave them out of the count and show them for information only (chosen).* The human still sees which open issues are taken in and by which items, and the menu offers *Contributions* only when an issue, pull request or alert needs a decision.
- *Have `clm intake` stop printing them.* Rejected: the lines are `orchestrator.triage/intake`'s stated contract and tell the human why an open issue is still open; the gap is in the skill's text, not the command.

## Design

- `plugin/triage-contributions` gains a bullet: each `issue <n> imported <slug>...` and `alert <n> imported <slug>...` line is an issue or alert the named items have already taken in; Claude shows it for information only and does nothing for it, since it stays open on the host until its item is resolved, when `clm intake` prints it as `resolved`. Its scenario's test also looks for `imported`.
- `plugin/triage-menu`'s *Contributions* entry says the `imported` lines aren't counted and are shown for information only.

The `verify <sha> imported` line belongs to `plugin/skills-run-verify` (change `20261107-slow-tests-after-land`, not yet built) and is left to it.

## Edge cases and workarounds

None known.

## Open questions

None.
