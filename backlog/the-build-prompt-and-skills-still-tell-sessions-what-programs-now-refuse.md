---
title: "The build prompt, the skills and the guide still tell sessions what the gate and verify now check"
kind: deferred
components: [orchestrator.run, plugin, init]
origin: 20261114-proof-is-real
---

Once `20261114-proof-is-real` is built, three texts say less or more than the programs do:

- `clm run`'s prompt and the build skill still tell a build to run mutation spot-checks by hand and to stay in its component's files as a matter of instruction. The gate refuses the second (`gate/own-files`) and `clm verify` does the first (`verify/mutation-pass`). The text is source (`prompt-text-is-source`): it should say what the gate refuses and what a `bug` item with `unproven` asks of the build that takes it, and drop the hand-run spot-checks.
- `/clm:propose`'s step for removals should name `clm coverage --obsolete-tests` as the program that lists the tests a removal leaves.
- The guide's page `docs/guide/how-it-works.md` says a marker is read "wherever it appears outside Markdown" and lists `path-ownership`'s exceptions as instructions. That page is a spec path, so its edit goes with the spec branch that relabels `path-ownership` and `proven-by-tests` in `ARCHITECTURE.md`.

The first two are code-only work, done on a code branch that resolves this item once the third has landed.
