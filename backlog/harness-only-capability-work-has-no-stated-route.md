---
title: "The method doesn't say how harness-only work on the capability tests lands"
kind: bug
components: [plugin, capability]
issues: [268]
---

Work that changes only the plumbing of a project's capability suite (its runner settings, the script that starts the system under test, ports, worker counts, values the tests repeat) changes no `CAPABILITIES.md` entry and nothing a capability test observes. `human-layer-proof` makes that harness the `capability` component's Design, which the AI owns, but nothing tells a session what route such work takes. One session wrote it as a requirement of the root component, whose build would then edit `capability`'s paths, which `path-ownership` forbids, and the change was parked.

Expected: spec-format's "Proving a capability", the propose skill and `human-layer-proof` say that harness-only work is never a requirement of any component. When it leaves `capability`'s Design true as written, it takes the small-change route (`small-change-no-claim`). When the Design must change, the Design edit lands first on a spec branch, and the same session follows with the code on a small-change branch, since a Design edit alone builds nothing (`node-reconciliation`). No new `path-ownership` exception, no harness requirements and no harness component.
