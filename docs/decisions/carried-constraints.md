---
title: "CLM's method constraints carry to every project from the clm it runs, in three tiers"
date: 2026-11-04
status: accepted
---

Some of CLM's architectural constraints are about CLM's own code (its guard, ledgers and web app); others govern how any project is built with it (`proven-by-tests`, `land-locally`, `expand-contract`, `no-dead-code`). The second kind reached a project only as an `## Enforced by clm` list `clm init` wrote once: frozen at the version that wrote it, kept by hand, and missing constraints the skills cite by name.

Each constraint that carries holds a `#### Carries` block in CLM's `ARCHITECTURE.md` naming its tier:

- **`method`**: CLM's machinery relies on the constraint itself (proof by tests, hash staleness, landing through `clm land`). A project can't replace it: without it, it isn't using CLM.
- **`replaceable`**: CLM relies on a guarantee, which a `#### Guarantee` block states, and the constraint names one technique for it. `expand-contract` is one way to keep `main` green while components move one at a time; feature flags are another. A project may replace it with its own constraint that keeps the guarantee.
- **`default`**: an opinion CLM's machinery doesn't rely on (`yagni`, `reuse-first`). A project may drop it. `redesign-before-workaround` is `method`, not `default`: `clm workaround` and the build's exit on a third workaround enforce it in every project.

The model compiles in CLM's `ARCHITECTURE.md`, so the carried set is the one the running `clm` enforces: `clm context` adds the entries in force to every bundle, and `clm guide constraints` prints them. A project names what it replaces or drops in a `#### Replaces` block of its own entry; the model refuses one that names a `method` constraint. Whether a replacement keeps its guarantee is a judgement: the `architecture-auditor` advises, its finding reaches the approval ask, and the human's approval decides, since an `ARCHITECTURE.md` edit always needs review and a model's judgment never gates (`deterministic-enforcement`).

A repository whose own `ARCHITECTURE.md` holds `#### Carries` blocks is the source, CLM's own, and reads the set from its own file, so a spec branch that rewords a carried constraint is judged by the words it proposes.

`clm guide constraints` is the one guide page generated rather than read from `docs/guide/`, the one exception to `guide-in-the-binary`: a copy of the carried entries under `docs/guide/` would be a second source. `ARCHITECTURE.md` stays a doc-only path for `ledger`'s code key; the tests that read the compiled copy hold `clm:reads ARCHITECTURE.md` markers, so an edit to it runs them at land (`doc-readers-marked`).

The carried text keeps the decision and constraint names it cites, which are CLM's own; the bundle section and the page say so, since the rule is the text they give and the names only say where the reasons are.

A project's `#### Replaces` names CLM's slugs, so a later `clm` that renames or removes a carried entry fails that project's `clm check` until it follows; CLM renames one as `expand-contract` says. A project could make itself the source by adding a `#### Carries` block, taking the carried set out of its bundles; no marker a program checks can tell CLM's repository from a project that copies it, so the edit's review and the auditor's report on it are the guard.

Rejected:
- **Copy at init, generated from the flags.** Fixes the hand-kept list but stays frozen once written.
- **Supply from the binary and refresh a copy in the file.** A second copy that drifts (`regenerate-dont-merge`, `spec-is-current`).
- **Let a project switch any constraint off with a warning.** A project without `proven-by-tests` or `land-locally` breaks the mechanisms CLM runs on, so the warning would describe a breakage rather than a choice.
- **Have the auditor's verdict gate a replacement.** Model judgment advises and never gates; the auditor makes sure the human is asked the right question.

## The rule in full

`ARCHITECTURE.md`'s entry `carried-constraints` states this in short (`human-layer-entries-stay-short`); the rule in full:

CLM's constraints that govern how any project is built with it SHALL reach every project from the `clm` it runs, never as a copy that can fall behind. Each such entry of CLM's `ARCHITECTURE.md` holds a `#### Carries` block naming its tier: `method`, which a project can't replace; `replaceable`, whose `#### Guarantee` block states what CLM relies on, which a project may replace with its own constraint keeping that guarantee; or `default`, an opinion a project may drop. A project replaces or drops one with a `#### Replaces` block in an entry of its own `ARCHITECTURE.md`, an edit a human reviews; the `architecture-auditor` advises whether a replacement keeps the guarantee (`deterministic-enforcement`).

**Scenario: replacing a method constraint**
- GIVEN a project whose `ARCHITECTURE.md` holds an entry with `#### Replaces` naming `proven-by-tests`
- WHEN `clm check` runs
- THEN it fails, naming the entry and `proven-by-tests`

**Scenario: a constraint CLM adds**
- GIVEN a project set up by an older `clm`, and a newer `clm` whose `ARCHITECTURE.md` carries a new `method` entry
- WHEN `clm context <component>` runs with the newer `clm`
- THEN the bundle holds the new entry, with no edit to the project's files

**Untested**
Whether a replacement keeps the guarantee it replaces is a judgement, which the auditor advises on and the human's approval decides.
