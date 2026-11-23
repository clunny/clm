---
title: "AGENTS.md still tells builds to run mutation spot-checks by hand"
kind: deferred
components: [orchestrator.run]
origin: 20261122-wave-4-orchestrator-by-the-rule
---

This repository sets `[verify] mutate`, so once `orchestrator.run/prompt-spot-checks` is built, `clm run`'s prompt leaves out the hand-run mutation spot-checks, while `AGENTS.md` step 4 still asks for them. Once that build has landed, a docs branch (its gate prints `spec`, and it lands on checks) edits `AGENTS.md` step 4, replacing

"Then mutation spot-checks: break each key rule on purpose and confirm a test fails, run as `clm run`'s prompt says: commit first, count only a mutant that compiles and fails a named test, `touch` each restored file, loop under `sh`, not `zsh`."

with

"Mutation is the mutation pass's, after the land (`mutation-after-land`): this repository sets `[verify] mutate`, so a build runs no spot-checks by hand."

and deletes this item. It changes no code, so `orchestrator.run`'s build does not take it: `AGENTS.md` should change only once the prompt it describes is built.
