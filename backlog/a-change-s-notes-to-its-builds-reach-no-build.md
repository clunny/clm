---
title: "A change's notes to its builds reach no build"
kind: bug
components: [plugin, orchestrator.run]
origin: 20261119-an-obsolete-marker-moves-in-three-steps
---

Step: proposing a change with `/clm:propose` whose builds must take a step in a set order, then building it with `/clm:build`.

What happened: the proposing session wrote what each build must do under a heading of its own in `change.md`. A build's prompt holds the diffs of its pending requirements and its context bundle, and nothing of `change.md`, so the build never saw the step, did the reverse, and the next component's build failed its land and was reverted.

Expected: either the propose skill says that a step a build must take is stated in a requirement, in the component's Design or in a backlog item naming the component, and never only in `change.md`; or the build prompt carries the part of a live change's `change.md` addressed to its component.

Rule involved: `ancestor-scoped-context`, `expand-contract`, `coupled-changes-sequence`.
