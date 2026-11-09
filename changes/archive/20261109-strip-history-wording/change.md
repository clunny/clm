---
status: done
---

## Intent

Rewrite the Contract and Design text that `clm health` flags as `health/history-words` in `check.coverage`, `ci`, `plugin` and `vcs`, together with the narration of past changes the spec-slimmers quoted beside it in the same sections. Specs say how the system is, not how it got there. This change touches no requirement and no scenario, so nothing has to be built.

## Problem

`clm health` reports five lines that narrate history:
- `check.coverage`: "used to stand in for".
- `ci`: two measurements dated 2026-11-08 and 2026-11-07.
- `plugin`: "today's command names".
- `vcs`: "honoured as before".

The same sections also narrate how the system got this way:
- `ci`'s Design tells how the build that moved the script to `nextest` was to proceed. That move has been made, and the `nextest-runner` requirement still states it.
- `ci`'s Design tells the incident that led to the capability packages' retry.
- `vcs`'s Design says `remove_worktree` "is moved here from `clm-run`".

Text like this goes stale, and every session reads it again.

## Approaches

- **Rewrite only the Contract and Design text (chosen).** Each sentence is dropped or put in the present tense, with its reason kept where it carries one. The requirements keep their versions, so no marker goes `behind` and the change is archived on its spec branch (`archive-in-spec-pr`).
- **Also reword the requirements that narrate.** These are `check.coverage/owner` ("as it does now") and the second sentence of `ci/nextest-runner`, the one-time migration instruction. Rewording them changes their versions and makes `check.coverage` and `ci` pending for a rebuild that only re-cites markers. Left for a slimming change that edits those requirements for other reasons too.
- **Act on the slimmers' full proposals.** These are larger cuts to Design that restates requirements, merged scenarios and removed requirements. Each needs judging component by component, so it is left for a change of its own.

## Outcome

`clm health` reports no `history-words` line for these four components. The edited paragraphs keep every rule and reason they had, apart from the dated measurements and the migration steps. One stray doubled comma in `vcs`'s Design is fixed in passing.
