---
title: "The build prompt and the skills still tell sessions what the land and verify now check"
kind: deferred
components: [orchestrator.run, plugin]
origin: 20261114-proof-is-real
---

Once `20261114-proof-is-real` is built, the prompt and skills say less or more than the programs do:

- `clm run`'s prompt and the build skill still tell a build to run mutation spot-checks by hand and to stay in its component's files as a matter of instruction. The land refuses the second (`land/build-holds-own-files`) and `clm verify` does the first (`verify/mutation-pass`). The text is source (`prompt-text-is-source`): it should say what the gate refuses and what a `bug` item with `unproven` asks of the build that takes it, and drop the hand-run spot-checks.
- `/clm:propose`'s step for removals should name `clm coverage --obsolete-tests` as the program that lists the tests a removal leaves.
- The build prompt should say what a `bug` item with `unproven` asks of the build that meets it: mend the test, or say in the resolving commit why it is sound, and delete the item with the `Resolves:` trailer.

Done on 2026-11-14, each on its component's own branch: `/clm:propose` now says a fix across components takes one claim per component, and the build prompt says to leave a requirement that changes a file `clm init` writes unbuilt with a question item.

The rest waits until `config/verify-here` is built, since the hand-run spot-checks are all there is until the mutation pass is on. This repository's `AGENTS.md` names them too, and is a spec path.

All of it is code-only work. It spans two components, so each one's part is done under its own claim, and the last deletes this item.
