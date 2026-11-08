---
status: applied
resolves: [the-guide-never-says-a-test-reading-the-project-s-own-files-needs-a-clm-reads-marker]
---

# The guide never says a test reading the project's own files needs a clm:reads marker

## Intent

States in the guide's page `ci-and-deployment` a rule an adopting project needs and no guide page or skill gave: a test that reads a file of the project's own tree under a doc-only path holds a `clm:reads <glob>` marker, since a land whose code was already checked runs only the tests whose markers match what the branch changes (`land/selected-tests`). The item also asks for `clm.toml`, whose comment-only changes count as checked under `land/settings-unchanged-is-checked`; that requirement is still unbuilt, and the guide describes CLM as released (`guide-from-docs`), so that sentence waits in the `kind: deferred` item `backlog/the-guide-doesn-t-say-a-test-reading-clm-toml-needs-a-marker` until it is built. The glossary's **Marker** entry now names `clm:reads` and `clm:remove-by` beside `req:` (`plain-terms`). Without the marker, a spec branch can land without running such a test and leave `main` failing. `method-in-the-plugin` asks that every working rule be said by a skill or a guide page; until now this one was written only in CLM's own `CLAUDE.md`, decisions and specs (`doc-readers-marked`).

## Problem

`clm land` counts a tree whose code key has a `checked` record as checked, and runs `[land] check` only for the readers `clm affected --readers` selects. After `20261107-an-upgrade-branch-that`, a land whose `clm.toml` differs only in comments or upgrade notes counts as checked too. An adopter's test with no marker is skipped on such lands, and nothing in the guide tells the adopter to write one.

## Approaches

- *A new subsection of `ci-and-deployment`'s "`clm land` is your CI" (chosen).* That section already says what a land runs; the rule is about which tests a land skips. The page is a hand edit on this spec branch, as guide pages are (`guide-from-docs`); `docs/guide/` counts as code for the code key, so the land runs the check.
- *A new `init` requirement whose test checks that the page names `clm:reads`.* Rejected (`yagni`): only the entry pages and the settings page have their prose pinned by a test, each for a reason of its own; `method-in-the-plugin` is the standing rule, and the architecture audit reads guide gaps.
- *A line in the `CLAUDE.md` that `clm init` writes.* Rejected: that `CLAUDE.md` points at the guide rather than restating it (`method-in-the-plugin`), and a template change needs a requirement and a build for a rule the guide already states.

## Outcome

`clm guide ci-and-deployment` says which lands skip tests, how a test names what it reads with `clm:reads <glob>` (with examples for `ARCHITECTURE.md` and `specs/**`), what goes wrong without the marker, and which tests need none. No requirement changes, so nothing is built.

## Edge cases and workarounds

None.

## Open questions

- Builder check and architecture audit: the audit found the `clm.toml` sentence described the unbuilt `land/settings-unchanged-is-checked` (`guide-from-docs`), now deferred to the item above, and that `clm:reads`, "doc-only paths" and `CLM_SELECTION` were undefined in the glossary (`plain-terms`): the glossary's **Marker** entry now covers `clm:reads`, and the page explains the paths and `CLM_SELECTION` in place instead of using them as terms. Nothing is left open.
