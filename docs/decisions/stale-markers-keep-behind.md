---
title: "A marker at an old version keeps its requirement behind, even beside a current one, and the requirement's build re-cites it wherever it sits"
date: 2026-11-02
status: accepted
---

`versioned-markers` made a requirement *current* when any marker cited its current version. A requirement's tests don't all sit in its component's `paths`: `crates/clm-check/tests` cites `check.coverage`'s requirements, and `crates/clm/tests/*.rs` cites most components' (`tests-count-where-cited`). When such a requirement was reworded, its component's build re-cited the tests in its own `paths` and left the others at the old version, since `path-ownership` said a build changes only its own `paths`. One current marker then made the requirement current, so `clm pending` listed nothing and no build was ever queued for the stale ones. The builds' reviewers saw the stale markers and could not touch them, and nothing tracked them. On `main` at the time 14 requirements were current with a marker at another version beside the current one. That broke `hash-based-staleness`, which says a requirement cited at an old version is behind and is built again.

Now a requirement is *current* only when every marker that cites it cites its current version. A marker at another version, or without one, keeps it `behind`. `clm coverage` lists those stale markers' locations, and `clm pending` lists the versions they cite. The build of the requirement's own component is queued. Its prompt names each stale marker's location and tells it to re-cite the test wherever it sits, once it has checked that the test proves the current text. `path-ownership` gains that exception: such a test is part of the cited component's build, as `tests-count-where-cited` and `fixes-count-where-cited` already count it.

This reuses the `behind` state, the queue and the build prompt, and adds no state, line shape or setting.

Known edge cases:
- A file that cites requirements of several components is edited by each one's build, on its own markers only. Two builds can then touch the same file at once, and `clm land`'s merge reconciles that as it does a removal's forced fix.
- A test in another component that proves the old text and fails the new one is fixed by the requirement's build. That is the same work a test in its own `paths` would need.
- The 14 requirements that are current today go `behind` when this lands, and their components are queued once each.

Rejected:
- **Counting a stale marker as pending on the component whose `paths` hold its file**, as an obsolete marker of a removed component is. That build's context doesn't hold the cited requirement's text (`ancestor-scoped-context`), so it could re-cite the test but not judge whether the test proves the new text. It would also need a new pending line shape and state for the views to count.
- **A `clm check` rule failing a stale marker beside a current one.** That would fail every tree between a reword and the build that re-cites, including the spec branch that rewords. It would also say nothing about which build fixes it.
