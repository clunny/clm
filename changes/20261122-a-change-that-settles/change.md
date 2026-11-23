---
status: applied
resolves: [a-change-that-settles-a-workaround-flag-leaves-it-to-fail-clm]
---

## Intent

`clm apply` deletes each `workaround` item the change's `resolves` names, as it already deletes each `question` item there (`apply/answered-questions`). A change that settles a build's flag then leaves nothing for `clm check` to fail on and nothing to delete by hand, so the route is one program step rather than a step each session has to remember. It resolves `backlog/a-change-that-settles-a-workaround-flag-leaves-it-to-fail-clm`.

## Problem

A change that settles a flag rewrites, re-scopes or completes the requirement the flag pins, so the requirement's version changes. The model rejects a flag whose version isn't current (`model/parse-backlog`), and only once `clm apply` has run, since a pending change's resulting graph leaves flags out (`apply/resulting-graph`). `clm apply` deleted only resolved `question` items, and the archive deletes the rest of `resolves` much later (`archive-on-sync`). So right after `clm apply`, `clm check` failed on the flag, and the session deleted it by hand in a commit of its own. The propose skill and the draft prompt mention the step, and the check's message says "re-record the flag or delete it", but nothing does it. Earlier changes all did it by hand, and an unattended draft that misses it fails late.

## Approaches

- **`clm apply` deletes a resolved flag** (taken). This is the same reasoning `apply-deletes-answered-questions` gave for questions, where it rejected "a check rule that makes the spec branch delete the question" because every author would do by hand what the command can do. A resolved flag is settled by the spec that is applied, and the archive would delete it anyway, so deleting it at apply changes only when it goes. It goes when the spec branch lands, and the build of the new text starts with no flag counted against it ("a rewritten requirement starts with a clean score", `workaround-flags`). That clean start also keeps `redesign-before-workaround`'s limit from counting a flag the change has already answered.
- **Only the check's message and the skills tell the session to delete it.** Both already do, and the bug happened anyway. Prose isn't a checked rule, and this one leaves a manual commit on every such change.
- **Delete a resolved flag only when the change modifies or removes the requirement it names.** This is narrower, but it needs a second condition in the rule and the code. A resolved flag on a requirement the change leaves alone, settled by a Design edit for example, would be deleted by the archive anyway. Deleting every resolved flag is simpler to state and to test (`yagni`).

## Outcome

- `apply/answered-questions` deletes resolved `question` and `workaround` items. Its other scenarios stay true: a `bug` and a `human-task` item stay, and a `resolves` entry naming no item deletes nothing. That last one covers a session that already deleted the flag by hand. A new scenario, *settled workaround flag*, covers a flag on a requirement the change modifies being deleted, with `clm check` passing after it. Apply's Contract line on `clm apply` names both kinds.
- A flag the change does not resolve is handled as before: the change deletes it, or re-records it at the new version when the workaround still applies, as `clm check`'s message says (*workaround flag on a rewritten requirement* under `apply/resulting-graph` still holds).
- Hand edits: `docs/decisions/apply-deletes-answered-questions.md` and `docs/decisions/workaround-flags.md` record the extension, and `docs/spec-format.md`'s `resolves` line names the workaround kind.
- `backlog/the-draft-prompt-and-propose-skill-have-a-session-delete-a-flag-clm-apply` (`kind: deferred`, `orchestrator.triage` and `plugin`) holds the wording of the draft prompt and the propose skill. That wording still describes the hand step, and it doesn't say that a flag kept at the new version must stay out of `resolves`. It is fixed text (`prompt-text-is-source`), so it is fixed by component once this is built. Until then it is harmless.

## Constraints

- `deleted_items` in `crates/clm-apply/src/lib.rs` is the one function that gives the items `clm apply` deletes. `resulting` takes its items from it too (apply's Design), so the graph a change is checked against already drops a flag that its apply deletes.
- No requirement is removed and no sentence holding `SHALL` is lost.

## Edge cases and workarounds

- A flag whose workaround still applies to the new text must not be named in `resolves`, or `clm apply` deletes it. Before this change, the archive deleted such a flag anyway, so nothing that worked before is lost. The deferred item above brings the draft prompt's and propose skill's wording into line.

## Open questions

None.
