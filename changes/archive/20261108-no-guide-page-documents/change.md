---
status: done
resolves: [no-guide-page-documents-clm-selection]
---

## Intent

Document `CLM_SELECTION` in the guide's page `ci-and-deployment`, so a project whose `[land] check` should run only the tests a branch can break learns the interface from `clm guide` rather than by dumping the environment from a probe script (`method-in-the-plugin`). A new subsection of "`clm land` is your CI" says when the variable is set (every land that runs the check, with `clm affected`'s output, or its `--readers` output on a tree whose code was already checked; `clm verify` once a run has passed), when it is unset (`clm affected` failed, `clm land --check-only`, `clm verify` with no passing run, a run by hand), the three line forms (`all`, `node`, `file`) and their order, and a short example script that maps `file` lines to test commands and falls back to every test when the variable is unset, empty or `all`. `CLM_AFFECTED`, a command override for tests, stays out of it.

The item's second part, whether the starter `scripts/clm-check.sh` that `clm init` writes should read the selection and what an existing project's script is offered (`upgrades-carry-migrations`), is a design question of its own, as the item says; it joins `backlog/init-check-starter-marks-no-steps`, the open idea about the same starter script, rather than being decided here.

## Problem

`land/selected-tests` and `verify` hand the check a selection, and `check.affected`'s Contract states the line format, but the guide named the variable only in passing, in the section on readers. An adopter can't read specs of CLM's own repository from their project; the guide is what `clm guide` carries.

## Approaches

- *A subsection of `ci-and-deployment` (chosen).* The page already says what a land runs and holds the readers section that mentions the variable. The page is a hand edit on this spec branch, as guide pages are (`guide-from-docs`), and everything it describes is built (`land/selected-tests`, `land/check-only`, `verify`, `check.affected`).
- *An `init` requirement whose test checks that the page names `CLM_SELECTION`.* Rejected (`yagni`), as `20261107-the-guide-never-says` rejected one for `clm:reads`: only the entry pages and the settings page have their prose pinned by a test, each for a reason of its own.
- *A section under `land` in `settings`.* Rejected: `settings` describes keys and their trade-offs; the variable is part of what the check command receives, which `ci-and-deployment` describes.
- *Have the starter script read the selection now.* Left to `backlog/init-check-starter-marks-no-steps`: the starter runs one line per language, and a selection mapping is language- and layout-specific, so it needs a design of its own, with the migration question.

## Outcome

`clm guide ci-and-deployment` documents `CLM_SELECTION` fully. No requirement changes, so nothing is built.

## Edge cases and workarounds

None.

## Open questions

None.
