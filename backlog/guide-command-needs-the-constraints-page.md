---
title: "guide-command lists the constraints page but doesn't Need model/carried-constraints"
kind: question
components: [init, model]
blocks: [init/guide-command]
---

`init/guide-command`'s current text makes `constraints`, the generated page, the last line of `clm guide`'s index, and its `index` scenario asserts that line. That page is built from `model`'s compiled copy of CLM's carried entries, which `model/carried-constraints` provides and nothing has built yet (`init/guide-constraints-page` is blocked on it). So a build of `init` can't make the scenario true, and the requirement has no `#### Needs` naming `model/carried-constraints` to say so.

A spec branch should add `model/carried-constraints` to `init/guide-command`'s `#### Needs` (or move the `constraints` page out of `guide-command` into `guide-constraints-page`, which already has the Need). Until then the build left `guide-command` unbuilt, with its markers at the old version, rather than cite it at a version whose scenario fails; the re-cites it lists wait with it.
