---
title: "A merge into a requirement of another slug is a move the change records"
date: 2026-11-24
status: accepted
---

A move was made by the deltas alone: a removal and an addition of the same slug on another component (`progress-follows-moves`), and the obsolete markers of the removed requirement wait on the added one (`moved-markers-wait`). A change that merges a requirement into one of another slug, on the same component or another, makes no such pair, so its obsolete markers waited on nothing. In wave 6, three `web` requirements merged into `web/worker-deploy`, which was blocked; their tests could be neither re-cited, which would make a blocked requirement current out of order, nor deleted, since no list named them, and the queue offered `web` for them to three builds in a row, each of which stopped.

So a change records each such merge in `change.md`'s `moves`, `{requirement: <removed>, to: <merged into>}` (`model/move-records`), and every reader of moves takes the pair as a move: the obsolete line waits on the target, a change's progress follows it, `moved_to` returns it, and `check/obsolete-tests-elsewhere` exempts a test its target's owner re-cites. Only the change's author knows which requirement took a removed one's sentences; the deltas can't say it, as they can for a same-slug move, so here the record says something nothing else does. The model checks the record against the change's own deltas (the one removed, the other added or modified), so a pair the change doesn't make is refused.

This amends `progress-follows-moves` and `moved-markers-wait`, whose move was the deltas' alone, and `a-question-holds-a-removed-requirement-s-unlisted-test`, which rejected a list of tests to re-cite written by the change as new machinery for a forgotten line. A merge isn't a forgotten line: the change knows the target when it is written, and a question would cost a build launched only to file it and a human's answer for that known fact. The question stays the route for a test the change forgot.

Considered:
- Inferring the target, as every blocked requirement the removing change adds or modifies on the same component. It holds a plain removal back behind unrelated work, and can't see a merge across components.
- Annotating the removal's heading in the delta. The delta format has more readers (`apply`, the gate, `rename`, the Worker) than `change.md`'s frontmatter, which the model alone reads.
- Reusing `renames`. A rename is replayed by `clm rename` and changes no text, and `renames` refuses two entries giving one new name, which a merge of several requirements is.
