---
title: "A split's cut can't pass clm survey when one upward import is removed by the change's own requirements"
kind: bug
components: [plugin, init]
issues: [245]
---

When a change carves a child out of a component, the propose skill checks the cut with `clm survey --components` against the tree as it stands and keeps changing the cut until the survey exits 0. A cut can be right except for an edge that the change itself removes, for example an import from a carved-out file to a parent module that one of the change's requirements replaces with a value the parent passes in. The survey still reports that edge as `survey/undeclared-edge`. The skill then pushes the session towards a worse cut, such as moving the parent's module into the child, and neither the skill nor `shaping-the-graph` says what to do.

Expected: a cut may keep an edge only when a named requirement of the same change removes it, and the method says how. That could be a field in the component map listing such edges, each with the requirement that removes it, which the survey would then not report. Or the skill and guide could say such a cut stands and the edges go in the pull request for the reviewer. Either way the child's build must leave no such edge, so `clm check` still finds the graph acyclic after the build. `splits-import-one-way` needs amending to cover it.
