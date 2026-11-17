---
title: "A build whose only work is deleting an unproven item is told its gate must print code"
kind: bug
components: [orchestrator.run, orchestrator.triage]
---

`clm run`'s build prompt tells the session to "confirm `clm gate --base origin/main` prints `code`" before it pushes and lands. A build whose pending list holds only a requirement that a `kind: bug` `unproven` item marks unbuilt, whose citing tests prove sound under hand mutation, does its work by deleting that item (with a `Resolves:` trailer) and changing no code, as the item itself says ("when they are already sound, delete this item"). Its gate prints `spec`, not `code`, so the prompt's step can't be met as written, though the branch is well formed and lands on checks.

Seen on `impl/clm` building `clm/doc-readers-marked@3125f2de` against `backlog/unproven-clm-eed96364`: the citing tests were already sound (twenty marker mutants each failed a named test), and changing them only to make the gate print `code` would make the next mutation pass re-judge them, a requirement whose rule lives in data the pass can't mutate (`changes/archive/20261116-the-mutation-pass-judges`).

The prompt should accept `spec` for a build branch whose only change deletes items it resolves, or say what such a build does instead. The fix-branch prompts `clm draft` writes (`crates/clm-triage/src/draft.rs`, `orchestrator.triage`) hold the same `prints code` step and meet the same case.
