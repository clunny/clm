---
title: "A stale delta's report shows what main changed since its base"
date: 2026-10-26
status: accepted
---

A delta's base is a hash, so `apply/stale-base` can say that `main` changed an element since the delta was written, but not how. Rebasing the delta needs judgment (`regenerate-applied-specs`): the author merges `main`'s new text into the delta's text and then records the new hash from `clm hashes`. Nothing made the first half happen. A delta whose hashes were updated but whose text wasn't passes `clm check`, `clm apply` and the gate, and when it lands it silently replaces the section with the older text, dropping what another change added while its requirements, which cite that text, stay.

`clm sync` first merges a stale section into the delta itself when the edits don't overlap (`stale-delta-three-way-merge`). When it stops on the sections left stale, it prints, for each such Contract, Design or requirement, the diff of that element from the delta's base to `origin/main`. It recovers the base's text from git: the newest version of the node's `SPEC.md` in `origin/main`'s history whose element hashes to the recorded base. The diff appears exactly when the author has to rebase, with an instruction to merge it before taking the new hash.

Alternatives considered:
- **A blocking check that the rebased text keeps every line `main` added.** It needs the old base's text after the base was updated, which only the delta file's own history holds, so `clm check` would have to read history it otherwise never reads, and it can't tell a dropped line from one the author meant to remove or reword. A rule that blocks intended edits needs an override, which is the same judgment the diff asks for.
- **Recording the base's text in the delta.** It would make the diff local, but every delta would carry a copy of each section it replaces, doubling what a reviewer reads.
- **Delta hunks instead of whole sections, merged by line.** A change to how deltas are written and applied, far larger than this bug, and `regenerate-dont-merge` prefers regenerating from what the author wrote over merging text.
