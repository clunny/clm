---
title: "path-ownership names no exception for a split child's build moving files out of its parent"
kind: bug
components: []
origin: 20261112-web-on-the-move
---

When a change carves a child out of a component (nested-splits), the child's build moves files out of the parent's paths, rewires the parent's entry module and re-points the parent's end-to-end test markers at the child's new requirement IDs. The architecture constraint path-ownership says a component's build changes only its own paths, with four exceptions, none of which covers the split move; only the decision nested-splits sanctions it. The architecture-auditor flags every split change against path-ownership as a result, and a builder reading the constraint alone would refuse the move. Expected: path-ownership's text names the split move (moving the carved-out files, rewiring the parent's imports and re-pointing moved markers) as an exception.
