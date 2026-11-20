---
status: done
---

## Intent

Have the build prompt name a test that a change lists to delete when the test's requirement stays, and let `upgrade` import `clm-git`. Both come from the builder check of `20261119-wave-1-by-the-rule`, wave 1 of phase 5 of `backlog/a-passing-check-proves-less-than-the-spec-claims`, and are built before that change lands so that none of its builds waits on them.

## Problem

- A change lists in `deleted-tests.txt` each test its builds are to delete (`cuts-are-what-a-program-lists`): a duplicate, or the test of a sentence a requirement lost. `clm land` lets a build lose a listed test. But `orchestrator.run/pending-prompt` names a listed test only under an `obsolete` requirement; under one that is `behind` the prompt lists the test's marker to re-cite, so the build keeps the test and the cut the human agreed never happens. 15 of the 17 tests wave 1 lists cite a requirement that stays.
- `vcs/one-git-layer` is about to read every crate, test modules included. `clm-upgrade`'s unit tests make a repository and a linked worktree with `git` themselves, and `upgrade` doesn't depend on `vcs`, so `check/undeclared-imports` refuses the import that would mend them.

## Approaches

- **Taken: the prompt lists a listed test under a `behind` requirement, as it does under an `obsolete` one.** One sentence and one scenario of `pending-prompt`, read from the same approved lists by the same comparison.
- **Not taken: writing each deletion into its component's Design.** It names tests in a spec, and every later wave would do it again.
- **Not taken: carrying both in wave 1's change.** `pending-prompt` needs `model/parse-backlog`, which wave 1 rewords, so the fix would be built after `model` and five components would each be built twice.
- **Taken: `upgrade` depends on `vcs`.** A component's tests import what its code may (`temporary-exemptions`); `clm-git` depends on nothing of the workspace.

## Outcome

- `orchestrator.run/pending-prompt` gains the sentence and the scenario "a listed test of a requirement that stays".
- `upgrade`'s `depends_on` is `[config, vcs]`. Its tests go through `clm-git` on a small code branch after this lands, since no requirement states how a test makes its fixture.

## Constraints

- Nothing is cut: no requirement, scenario or SHALL sentence goes.
- `orchestrator.run` is built once for this and again in its own wave of phase 5.
