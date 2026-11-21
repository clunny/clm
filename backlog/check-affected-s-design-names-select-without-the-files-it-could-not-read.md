---
title: "check.affected's Design names select without the files it couldn't read"
kind: bug
components: [check.affected]
---

**The problem.** `specs/check/affected/SPEC.md`'s Design gives `select(graph, changed, markers, all) -> Selection`, while its own next sentences say the scan keeps the listed files it can't read as UTF-8 apart and `select` takes them for the exemption rule alone. The build of `affected-files` at `72f3baff` gave `select` that list as a parameter, `select(graph, changed, markers, unread, all)`, so the signature the Design writes no longer exists.

**What is needed.** A spec branch that rewrites the signature in the Design to name the unread files. No code changes.
