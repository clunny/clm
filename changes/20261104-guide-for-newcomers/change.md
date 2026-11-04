---
status: applied
---

# The guide's entry pages stay written for a newcomer

## Intent

The quickstart and Using CLM were rewritten for a newcomer (landed on `main` as a guide-only branch): the quickstart takes a newcomer from install to a first build and names the four workflows, Using CLM describes those workflows as the human sees them, and the mechanisms (spec branches, review rules, landing, proof, `clm` commands) moved to a new page, How it works. Nothing in the spec states that shape, so the next edit that adds mechanism to an entry page goes unnoticed. This change adds a requirement to `init`, which owns `clm guide`'s pages, that a test checks the shape against, and has `/clm:init`'s closing summary point at How it works beside Using CLM, since "how a change flows and where decisions go" now lives there.

## Problem

Guide rules today cover links, page order, the glossary and removed skill names, never what a page is for. The quickstart had grown history and setup detail a newcomer doesn't need, and Using CLM mixed the workflows with the CLI and branches underneath. `plugin/method-pointers` makes `/clm:init` point at `clm guide using-clm` for how a change flows, which that page no longer explains in full.

## Approaches

1. **A narrow, checkable rule (chosen).** The quickstart names the install commands and the workflow skills and links Using CLM and How it works; neither entry page names a `clm` command in a code span; Using CLM links How it works. Wording stays free; the one line that keeps mechanisms out is mechanical.
2. **A prose rule with no test** ("entry pages are written for a newcomer"). Untestable, so `proven-by-tests` would leave it `#### Untested` and nothing would catch drift.
3. **Word or section budgets per page.** Catches growth but not mechanism creeping in, and fights every legitimate edit.

## Outcome

- `init/guide-entry-pages`: the shape of `quickstart` and `using-clm`, proved by a test in `clm-init` that reads `docs/guide/`.
- `plugin/method-pointers`: `/clm:init`'s closing summary points at `clm guide using-clm` for the workflows and `clm guide how-it-works` for how a change flows and where decisions go.
- Decision `guide-entry-pages-for-newcomers`; `first-graph-teaches-shape` amended for the second pointer.

## Constraints

- `guide-links-inside` still holds: How it works is a guide page, so the links resolve.
- `docs/guide/` counts as code (`guide-in-the-binary`, `doc-readers-marked`), so a guide edit reruns the test with no `clm:reads` marker.
