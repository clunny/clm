---
title: "An item can't hold back an obsolete requirement whose removing change is `done` but not archived"
kind: question
components: [model, check.coverage]
choices:
  - "Let `done` through too"
  - "It can't happen: leave it"
---

`model/parse-backlog` lets a `blocks` entry name a requirement removed by a change outside the archive only while that change's `status` is `applied` or `implementing`. `check.coverage` reads a requirement as `obsolete` while any change outside the archive removes it, whatever the change's status (`crates/clm-coverage/src/coverage.rs`, `removed()`), and counts `done` among the changes that move a requirement.

So if a change at `status: done` sits outside `changes/archive/` with a delta that removes `a/gone`, which a test still cites, `clm pending` lists `a/gone` as `obsolete`. A build that can neither delete nor re-cite that test files the `question` item that `orchestrator.run/pending-prompt` asks for, with `blocks: [a/gone]`, and loading rejects it, so `clm check` fails and the build has no way to hold its line.

The question for a spec change: can a `done` change stay outside the archive while a test still cites a requirement it removed? If it can, `parse-backlog` should name `done` beside `applied` and `implementing`. If it can't, the requirement could say why `done` is left out. Found by the code review of the `impl/model` build of `model/parse-backlog@18344ef2`.
