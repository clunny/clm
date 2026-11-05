---
title: "guide-command lists the constraints page but doesn't Need model/carried-constraints"
kind: question
components: [init, model]
blocks: [init/guide-command]
---

`init/guide-command`'s current text makes `constraints`, the generated page, the last line of `clm guide`'s index, and its `index` scenario asserts that line. That page is built from `model`'s compiled copy of CLM's carried entries, which `model/carried-constraints` provides and nothing has built yet (`init/guide-constraints-page` is blocked on it). So a build of `init` can't make the scenario true, and the requirement has no `#### Needs` naming `model/carried-constraints` to say so.

A spec branch should move the `constraints` page out of `guide-command` into `guide-constraints-page`, which already has the Need and whose page has no file (`guide-command` also promises each page's Markdown "exactly as its file is at build time"); adding only the Need to `guide-command` would leave that contradiction. That branch deletes this item, which releases `guide-command`. Until then `guide-command` stays `behind`, its eight markers at the old version, rather than cited at a version whose `index` scenario fails.
