---
status: done
---

## Intent

Phase 4 of the plan in `backlog/a-passing-check-proves-less-than-the-spec-claims.md`: the human's answers to three questions the `commands-in-crate` builds filed, given on 2026-11-15. The root's Design gains two rules so the remaining builds stop asking them, and the four workaround flags those builds raised are accepted.

It adds and removes no requirement, so it is archived in its own branch.

## Problem

Four builds each left tests in the root's files that only a `clm` process, or another component's command, can show, and each flagged a workaround. The third flag brought `capability/prove-by-tests` to its limit, stopped `init`'s build and asked whether the design is wrong (`redesign-before-workaround`). Every remaining build would do the same. A fifth build asked whether scenarios that name the `clm` binary must be reworded now that its tests call the crate.

## Approaches

- **Keep the design and say so where builds read it.** Taken, by the human. The flags share one cause, already answered: such a scenario is the root's (`commands-live-in-their-crates`). The root's Design, which every build's bundle holds, now says that leaving such a test with a `question` item is the design.
- **A test package that holds the binary and is no component's, or dependencies for tests alone.** Not taken: the first is the root under another name, and `components-own-their-tests` rejected the second.
- **Reword scenarios to name the crate's call.** Not taken: a scenario states what a person sees, and a function's name in a requirement is what the plan's phase 5 removes.

## Outcome

- The root's Design: a test of another component's command on what this component made stays with a `question` item and no workaround flag; a scenario that names `clm <command>` is proven by the crate's call and not reworded.
- `backlog/commands-in-crate-moves-leave-tests-in-the-root` is answered and deleted, which unblocks `init/commands-in-crate`.
- `backlog/prove-by-tests-has-three-unsettled-commands-in-crate-workarounds`, the same question asked by `land`'s build, is answered and deleted with it, which unblocks `land/commands-in-crate`; `backlog/five-land-tests-observe-only-the-clm-process` carries the answer.
- `backlog/seventeen-init-tests-can-t-move-into-its-crate` and `backlog/effort-scenarios-still-name-the-clm-binary` carry the answers.
- The workaround flags of `config`, `check.unreferenced`, `init` and `land` on `commands-in-crate` are accepted by the human and deleted.

## What it removes

Nothing.
