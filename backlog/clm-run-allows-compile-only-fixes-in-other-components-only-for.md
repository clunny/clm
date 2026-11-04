---
title: "clm run allows compile-only fixes in other components only for removals, not for an added field built by literal"
kind: bug
components: [orchestrator.run]
---

Step: designing a change whose requirements add a field to a public struct of one component (a library crate) that another component's code constructs by struct literal, here the root binary and its tests.

What happened: the builder check found that the providing component's build breaks the workspace typecheck as soon as it adds the field, since every struct literal in the other component must name it. The `clm run` prompt (`orchestrator.run`'s removal rule, `removal-breaks-dependant`) allows the "compile-only and test-fixture fixes the removal forces" in another component, but only for a removal. An addition that forces the same kind of mechanical fix (setting the new field to its empty value) has no stated route: a `#### Needs` can't order it, because the consumer needs the field to exist first, and the provider's build may not edit the consumer.

Expected: the run prompt's compile-only exception should cover fixes any Contract change forces, additions included, where the other component's code only names the changed item and its behaviour doesn't change; or the propose builder check should name the pattern so the change states it.

Rules involved: architecture/path-ownership, contract-removal-needs, removal-breaks-dependant.

The same limit is written in `ARCHITECTURE.md`'s `path-ownership`, whose first exception names only a removal, so the fix is a reviewed edit there as well as to the `clm run` prompt. `20261104-detectors-skip-in-the-root` designed around it for its case.
