---
title: "A line reusing a request's id with an earlier time removes the request from the fold"
kind: bug
components: [ledger]
origin: 20261121-wave-3-land-by-the-rule
---

`ledger/effort-records` keeps, of differing lines that share an `id` and are not all `effort` counts, the least by `at`, `writer` and sorted text. A `changes_requested` record's `id` is visible to everyone, so any writer can append to their own ledger a line of any kind with that `id` and an earlier `at`, and the fold then holds that line instead of the request. The request disappears from `change_requests`, `clm land` no longer holds the branch, and the web app no longer shows it. The same goes for any record a land or a review reads (an approval, a check). Random 128-bit `id`s make an accidental clash impossible, so only a deliberate line does this. Right: a line never displaces a record another writer wrote. Lines that share an `id` are one record only when they share a writer (or are all `effort` counts, whose `id` is `effort_id(session, branch)` by design). Mending it changes the choice wave 2 stated, so it needs a spec change to `effort-records`, and possibly `fold`.
