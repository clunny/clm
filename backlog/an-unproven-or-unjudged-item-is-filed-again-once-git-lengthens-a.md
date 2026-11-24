---
title: "An unproven or unjudged item is filed again once git lengthens a commit's short sha"
kind: bug
components: [verify]
---

`verify/files-unproven` names each item `backlog/unproven-<component>-<short sha of R>.md` (and `unjudged-...` of `R'`), the short sha as `git rev-parse --short` gives it, and skips filing when a commit after R added that path. git lengthens its abbreviations as a repository grows, so once `rev-parse --short R` returns more digits than when the item was filed, the check looks for a path nobody added and files the item again; if a build had deleted it, it comes back. The `unjudged` run is recomputed on every `clm verify --due` while the requirement stays in `left`, and `unproven` records stay in the ledger, so both can hit it. Fix needs a spec change: name items with a fixed-length abbreviation (e.g. 8 hex digits), or match the added path by prefix as `kept-log` matches logs. Found by /code-review on impl/verify's build of files-unproven@ec5a9e04.
