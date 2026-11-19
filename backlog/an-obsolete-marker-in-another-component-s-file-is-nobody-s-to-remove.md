---
title: "An obsolete marker in another component's file is nobody's to remove"
kind: question
components: [check.coverage, orchestrator.run]
origin: 20261119-the-cuts-rules-written-down
choices:
  - "Count it as pending on the file's owner"
  - "Leave it: only an exempted file can hold one"
---

When a change removes requirement `b/gone` and a test in a file component `c` owns still cites it, `check.coverage/pending-report` keeps `b/gone` `obsolete` on `b`. Once the root sets `wiring`, `clm land` refuses `b`'s build an edit to `c`'s file, so the build prompt lists the marker apart and tells the build to leave it and add a `question` item (`orchestrator.run/pending-prompt`). That item can block nothing, since `model/parse-backlog` rejects a `blocks` entry naming a requirement the spec doesn't hold. So `b` stays pending and ready, the queue offers it again, and the removing change never archives. No requirement gives `c`'s build the removal.

It can arise only where an exemption from `check/tests-cite-their-owner` covers the file, since `clm check` otherwise fails a test that cites another component. Should such a marker count as pending on the file's owner, as `pending-report` already counts a removed component's markers, so that `c`'s build removes or re-cites it?
