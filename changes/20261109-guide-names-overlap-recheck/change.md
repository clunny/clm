---
status: applied
resolves: [guide-names-overlap-recheck, the-guide-doesn-t-say-a-test-reading-clm-toml-needs-a-marker]
---

## Intent

The guide's page `ci-and-deployment` describes what v0.23.0 shipped for lands and that no guide page states: a land whose branch's head was checked whole re-checks only the overlap with what `main` gained since, and nothing when there is none (`land/overlap-recheck`), with its `re-check:` line and the `landed` record's `recheck`; what the carried constraint `land-checks-stay-fast` asks of a project's tests; `CLM_LAND_DIR` (`land/land-dir-env`); and that a land whose `clm.toml` changes only in comments or upgrade notes counts as checked, so a test reading `clm.toml` holds `clm:reads clm.toml` (`land/settings-unchanged-is-checked`). Both items were deferred until those requirements were built and released (`guide-from-docs`); all three are current and cited by tests in `v0.23.0`, and the human answered to draft them together.

## The human's answers

> ## guide-names-overlap-recheck, the-guide-doesn-t-say-a-test-reading-clm-toml-needs-a-marker
>
> Question: both conditions are met since v0.23.0, and both edit the guide's ci-and-deployment page. Draft them now?
>
> Answer: "Draft together (Recommended)": one change adding both to the ci-and-deployment page.

## Problem

`method-in-the-plugin` asks that every working rule an adopting project needs be said by a skill or a guide page. Since v0.23.0 a land may run no test at all, and its verdict rests on the project's tests declaring what they read; the rule that asks this of them (`land-checks-stay-fast`) reaches the project through `clm guide constraints`, but nothing in the guide explains which lands run what, how to read the `re-check:` line, or what to do with `CLM_LAND_DIR`. Only the upgrade note said it, once. The guide's `clm:reads` subsection still covered the doc-only paths alone, though a `clm.toml` changed only by `clm upgrade --apply` now lands the same way.

## Approaches

- *Hand edits to `ci-and-deployment` (chosen).* A new subsection "Re-checking only the overlap" after "A build's own check counts for its land", which it extends, holding the `re-check:` lines, `recheck` and what `land-checks-stay-fast` asks; a subsection "Where a land builds" for `CLM_LAND_DIR`, with a cargo example matching the upgrade note; a `CLM_SELECTION` bullet for the overlap; a paragraph on `clm.toml` in "Tests that read your specs and docs"; and two sentences corrected that the overlap made untrue (that the tree reaching `main` is the tree checked, and that only spec-only lands skip an integration-gating check). Guide pages are hand edits on a spec branch (`guide-from-docs`), as `20261107-the-guide-never-says` did.
- *A requirement of `init` pinning the page's text with a test.* Rejected (`yagni`), as in `20261107-the-guide-never-says`: the guide's prose isn't pinned by tests except the entry and settings pages.

## Outcome

`clm guide ci-and-deployment` says when a land re-checks only the overlap and when it runs nothing, what it prints and records, what the project's tests owe that verdict, where a land's check should put its build output, and that tests reading `clm.toml` need a `clm:reads clm.toml` marker. No requirement changes, so nothing is built.

## Constraints

- The page describes only released behaviour (`guide-from-docs`). It says `clm land` still sets `CARGO_TARGET_DIR` and `CARGO_INCREMENTAL` "for now", as v0.23.0's upgrade note does, so the text stays true when `backlog/land-still-sets-cargo-variables` is built; that build's change updates the sentence.

## Edge cases and workarounds

None.

## Open questions

- Architecture audit: under `plain-terms`, the page used the overlap re-check, the `re-check:` lines, the `recheck` values and `CLM_LAND_DIR` without glossary entries, and the glossary's **Marker** entry limited `clm:reads` to specs and docs. Fixed: the glossary gains **Overlap re-check** and **`CLM_LAND_DIR`**, and **Marker** names `clm.toml`. Builder check: no statement wrong; it asked that the page not read as `clm verify` explaining a culprit's failure (`verify/culprit-recheck` says nothing of the cause), that it name when a land takes no land lock (a caller-set `CARGO_TARGET_DIR`), and that the cargo sentence say `clm land` sets them only when the caller hasn't. All three are reworded. Nothing is left open.
