---
status: proposed
resolves: [triage-s-wrap-up-step-doesn-t-say-how-to-archive-the-changes-clm]
---

## Intent

Have `/clm:triage`'s wrap-up archive the changes `clm wrap-up` lists as `complete <id>`, the way `/clm:build`'s wrap-up does: on one `impl/archive/<id>` branch cut from `origin/main`, where `clm rebase` archives every complete change and deletes the items they resolve, landed with `clm land`.
