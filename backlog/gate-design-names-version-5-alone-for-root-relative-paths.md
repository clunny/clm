---
title: "Gate's Design says a top-level spec-path name outside the root is `code` under version 5 alone"
kind: bug
components: [gate]
---

In `specs/gate/SPEC.md`, the Design's **Rules versions** paragraph says a branch editing a spec-path name at the repository's top outside a subdirectory root, such as its own `CLAUDE.md`, "is `spec` under the lower versions and `code` under 5". Version 6 reads paths relative to the root as 5 does (`rules-version`: "Under versions 5 and 6 the gate SHALL read the paths a branch changes as `paths-under-the-root` states"), and the code agrees (`Root::of` takes the old reading only for `rules < 5`). The sentence before it already says "under 5 and 6". The Design should say "`code` under 5 and 6". Only the Design text is wrong, so the fix needs a spec branch. The `gate` build that brought `rules-version` to version 6 found it.
