---
title: "The draft prompt and propose skill have a session delete or re-record a flag that clm apply now deletes"
kind: deferred
components: [orchestrator.triage, plugin]
origin: 20261122-a-change-that-settles
---

Once `20261122-a-change-that-settles` is applied, `clm apply` deletes each `workaround` item the change's `resolves` names (`apply/answered-questions`). Two texts still describe the hand step it replaces, and neither says that a flag kept because its workaround still applies to the new text must stay out of `resolves`, since `clm apply` deletes a resolved flag and the archive would delete a re-recorded one (`archive-on-sync`):

- The draft prompt, `crates/clm-triage/src/draft.rs` (the sentence starting "A claimed `workaround` item is a build's flag against a requirement"): it says the change resolves a claimed flag by "deleting the flag, or by re-recording its `requirement` at the new version", while the prompt's first step puts every claimed item in `resolves`. It should say that `clm apply` deletes a claimed flag the change settles, and that a flag whose workaround still applies to the new text leaves `resolves` and is re-recorded after `clm apply`.
- The propose skill, `plugin/skills/propose/SKILL.md` steps 5 and 7: they say to delete each flag on a requirement a delta modifies or removes. They should say `clm apply` deletes the flags the change names in `resolves`, and that the session deletes or re-records by hand only a flag the change doesn't resolve.

Both are fixed text (`prompt-text-is-source`), with no test of their wording: fix each by component, with `clm claim <component> --fix backlog/<slug>` and the trailer `Resolves: backlog/the-draft-prompt-and-propose-skill-have-a-session-delete-a-flag-clm-apply` on the second fix's commit, once `apply/answered-questions` is built at its new version. Until then the texts are harmless: a flag a session deletes by hand is a `resolves` entry naming no item, which `clm apply` passes over.
