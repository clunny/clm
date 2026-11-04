---
title: "A code fix belongs to the nodes a changed test cites, as well as to the node whose paths hold it"
date: 2026-10-26
status: accepted
---

A bug whose fix changes no spec deletes its item in the code PR that fixes it (`code-fix-resolves-bug`): `propose` claims a node the bug names with `clm claim <node> --fix backlog/<slug>`, and the gate passes the deletion when the PR's code touches one of the bug's `nodes`. "Touches" meant a changed file matched a node's `paths`. Some nodes' tests don't live in their `paths`: `plugin`'s skill tests are `crates/clm/tests/plugin.rs`, in the root `clm`'s, because Cargo gives the binary only to the root package's tests (`tests-count-where-cited`). A wrong test of `plugin`, the very case the decision named, changed only the root's files, so it couldn't take the code-fix path unless the item also named `clm`, and fell back to a spec PR.

Now a code file belongs, for a code fix, to every node whose `paths` match it and to every node it cites with a `req:<node>/` marker in its text at the merge base or at `HEAD`. The gate's exemption reads it so (`gate/backlog-additions`), and `propose` claims the node the test cites (`plugin/propose-code-fixes`). `clm claim --fix` doesn't change: it already claims any node the bug names. The attribution is the one `tests-count-where-cited` uses for size, a marker naming the node whose requirement the test proves, so one rule says which node a test is part of. The gate searches the file's text for the literal prefix rather than parsing markers, which keeps it out of `check`'s scanner: a dotted child's ID (`plugin.hooks`) never contains its parent's `req:plugin/`, and fixtures that build their markers from pieces cite nothing, as they cite nothing to `check`.

What still bounds the exemption: only bug items, the item's own nodes, and a file whose markers already cite that node, at the merge base or in the fix. A PR could add a `req:<node>/` marker to an unrelated file to qualify, but a marker must cite a requirement that exists or `check` fails the tree (`dangling-markers`), so the test then proves that node; and deleting a bug item is an `AUTO` edit, merged on checks with no human review either way.

Alternatives considered:
- **Have `propose` add the owning node to the item's `nodes` before claiming.** The claim reads the item on `origin/main`, so the edit needs its own spec PR first, the cost `code-fix-resolves-bug` exists to remove. Adding it when the item is filed doesn't work either: the reporter rarely knows which file the fix will change.
- **Carve the test files into the cited nodes' `paths`.** Rejected for the same reasons as in `tests-count-where-cited`: the root's globs can't express an exclusion, and `paths` stay disjoint.
- **Attribute a test file only by its markers at `HEAD`.** A fix that deletes a wrong test, or removes its last marker, would then belong to no node; reading either version keeps it with the node it proved.
