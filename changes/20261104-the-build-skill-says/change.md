---
status: applied
resolves:
  - the-build-skill-says-clm-next-waits-on-settled-dependencies
---

# The build skill says what clm next waits on

## Intent

`skills/build/SKILL.md` tells Claude that `clm next` prints the components "whose dependencies are all settled". The queue stopped ordering by `depends_on` long ago: only a requirement's `#### Needs` and the backlog items that block it hold work back (`needs-alone-order`, `orchestrator/next`). This adds a `plugin` requirement that the skill says what `clm next` really waits on, naming `needs-alone-order` and `component-reconciliation`, and never says it waits on settled dependencies, so the build that meets it fixes the sentence and a test keeps it fixed.

## Problem

A session following the skill may wait for a dependency to build before launching a component, or explain an empty queue by an order CLM no longer applies. No requirement of `plugin` states what the build skill says `clm next` prints, so the sentence drifted with nothing to catch it, and a code-only fix would restore behaviour no rule states (`code-fix-needs-a-written-rule`).

## Approaches

- **Fix the sentence on a code branch alone.** Cheapest, but nothing written says the skill must describe the queue correctly, so the same drift could come back unseen. Rejected by `code-fix-needs-a-written-rule`.
- **Drop the explanation from the skill and just say `clm next` prints the ready components.** Nothing left to drift, but a session reading the skill then has no word for why a component with unbuilt dependencies is listed, and is the one most likely to second-guess it. Rejected.
- **A requirement that the skill says what holds a component back, by the rule's names, and a test that bans the old phrase (chosen).** The description names `needs-alone-order` and `component-reconciliation`, so a reader can look the rule up, and the test fails if the old ordering comes back.

## Outcome

- `plugin/build-says-what-is-ready`: the build skill's `clm next` step says `clm next` prints the components with a pending requirement, unclaimed, that no `#### Needs` or blocking backlog item holds back, says `depends_on` doesn't order builds, names `needs-alone-order` and `component-reconciliation`, and doesn't say `dependencies are all settled`; nor does step 4 say a landed component `settles the components that depend on it`, the same stale order, which the builder check found.
- The build keeps the sentences `build-finishes-builds`' test reads (the latest `clm next`, run `clm next` again) whole while it rewords step 1.

## Constraints

- `needs-alone-order`: the skill's text follows `orchestrator/next`, which it never restates beyond the one sentence.
- No edge cases or workarounds are known.
