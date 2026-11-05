---
title: "CLM's method constraints carry to every project from the clm it runs, in three tiers"
date: 2026-11-04
status: accepted
---

Some of CLM's architectural constraints are about CLM's own code (its guard, ledgers and web app); others govern how any project is built with it (`proven-by-tests`, `land-locally`, `expand-contract`, `no-dead-code`). The second kind reached a project only as an `## Enforced by clm` list `clm init` wrote once: frozen at the version that wrote it, kept by hand, and missing constraints the skills cite by name.

Each constraint that carries holds a `#### Carries` block in CLM's `ARCHITECTURE.md` naming its tier:

- **`method`**: CLM's machinery relies on the constraint itself (proof by tests, hash staleness, landing through `clm land`). A project can't replace it: without it, it isn't using CLM.
- **`replaceable`**: CLM relies on a guarantee, which a `#### Guarantee` block states, and the constraint names one technique for it. `expand-contract` is one way to keep `main` green while components move one at a time; feature flags are another. A project may replace it with its own constraint that keeps the guarantee.
- **`default`**: an opinion CLM's machinery doesn't rely on (`yagni`, `reuse-first`). A project may drop it.

The model compiles in CLM's `ARCHITECTURE.md`, so the carried set is the one the running `clm` enforces: `clm context` adds the entries in force to every bundle, and `clm guide constraints` prints them. A project names what it replaces or drops in a `#### Replaces` block of its own entry; the model refuses one that names a `method` constraint. Whether a replacement keeps its guarantee is a judgement: the `architecture-auditor` advises, its finding reaches the approval ask, and the human's approval decides, since an `ARCHITECTURE.md` edit always needs review and a model's judgment never gates (`deterministic-enforcement`).

A repository whose own `ARCHITECTURE.md` holds `#### Carries` blocks is the source, CLM's own, and reads the set from its own file, so a spec branch that rewords a carried constraint is judged by the words it proposes.

Rejected:
- **Copy at init, generated from the flags.** Fixes the hand-kept list but stays frozen once written.
- **Supply from the binary and refresh a copy in the file.** A second copy that drifts (`regenerate-dont-merge`, `spec-is-current`).
- **Let a project switch any constraint off with a warning.** A project without `proven-by-tests` or `land-locally` breaks the mechanisms CLM runs on, so the warning would describe a breakage rather than a choice.
- **Have the auditor's verdict gate a replacement.** Model judgment advises and never gates; the auditor makes sure the human is asked the right question.
