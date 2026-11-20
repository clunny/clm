---
status: applied
resolves: [gate-design-names-version-5-alone-for-root-relative-paths]
---

## Intent

`gate`'s Design names the rules versions that read paths relative to the CLM root correctly in both places: a branch editing a spec-path name at the repository's top outside a subdirectory root, such as its own `CLAUDE.md`, is `code` under versions 5 and 6, not under 5 alone (`backlog/gate-design-names-version-5-alone-for-root-relative-paths`). A Design edit only, so nothing is built.

## Problem

The **Rules versions** paragraph says such a branch "is `spec` under the lower versions and `code` under 5". Version 6 reads paths as version 5 does: `rules-version` states "Under versions 5 and 6 the gate SHALL read the paths a branch changes as `paths-under-the-root` states", the code takes the old reading only for `rules < 5`, and the same sentence already says the root's spec files are `spec` "under 5 and 6". The clause is a slip from `20261119-a-placement-phrase-wrapped`, which added version 6 and missed it.

## Approaches

- **Taken: say "under 5 and 6"** in the Design, matching the requirement and the code.
- **Not taken: "under 5 and later"**: every later version's path reading is a choice its change makes and states, as 6's did; naming the versions keeps the paragraph exact.

## Outcome

- `gate`'s Design: the **Rules versions** paragraph says a branch editing a spec-path name at the repository's top outside the root is `code` under 5 and 6.

No requirement or scenario changes, so no component becomes pending and no build follows; the code already agrees.

## Constraints

- Edge cases and workarounds: none.
