---
title: "Triage folds the HARD changes its drafts leave into one spec branch, approved once"
date: 2026-10-27
status: superseded
superseded_by: yes-no-review
---

A `/clm:triage` run drafts each group in its own subagent, and every draft whose tier came out `HARD` used to open its own spec branch and page, which the human approved one at a time. One run of 34 bug groups left 23 HARD branches. After the first landed, 9 went stale against each other, since they edited the same Designs and requirements, and each needed a merge and a fresh approval (`pinned-approval`). The human said they should have been one PR, approved once, which is what `CLAUDE.md` already asks of each piece of design work.

So a draft whose tier is `HARD` opens no page: it stops after its last push and closes with `proposed <id> HARD`. Once every draft has returned, the `triage` skill runs `clm draft --fold <id>...` with those IDs and gives the prompt it writes to one more background subagent, the fold. With one change the fold only opens its page. With several, it cuts `spec/<first ID>-fold` from `origin/main` and brings each draft's change over as one commit of its diff outside `specs/` (`git diff origin/main...origin/spec/<id>`, applied with `git apply --3way`), in `clm triage`'s order. It never merges: `clm sync` rebases without keeping merge commits, so a merge's conflict resolutions would be lost at the next sync. Each change keeps its ID, its `change.md` and its deltas: one branch carries several changes, which the gate already rates per change directory, `clm sync` already re-applies in ID order, and `clm land` already records together (`landed`'s `changes`). Where two changes modify the same element, `clm sync` reports the later one's delta stale against the earlier, and the fold merges the earlier edit into it, which is the merge the human's sessions did by hand after each land, done once and before the approval. The fold then runs the builder check on the elements where two changes met, writes one page body holding each change's sections, opens one page and stops. The skill asks the human once, through the guard (`approval-policy`), lands the branch, and runs `clm draft --finish` on each folded draft, which now also deletes a draft's branches once its change is on `main`.

**Why keep each change, not rewrite them into one.** Folding the drafts' text into a single new change would need every delta for a node rewritten as one, and the drafts' changes would never reach `main`, so nothing would tell their branches, worktrees and items apart from abandoned work. Kept whole, each change reaches `main` under its own ID: `clm draft --finish` finds it there, removes the draft's worktree and deletes its branches, and the drafts' branches keep holding their items (*proposed*) right up to the land, with no change to what holds an item (`propose-claims`). Nothing is pushed to a draft's branch, so a fold that stops loses nothing: each draft still shows as `open` for `/clm:propose`.

**Why a fold subagent.** The merge of overlapping sections is design work, which the triage session never does in its own context, and the fold's prompt is CLI text, as the draft's is (`deterministic-enforcement`: the plugin holds no rule).

Rejected:
- **One HARD branch per draft, approved in order (the old rule).** The cost above: N approvals, and each land staling the rest.
- **Drafts in sequence on one shared branch.** Loses the parallelism `backlog-triage` is for, and a draft's failure would block the rest.
- **Folding every draft, AUTO and SOFT too.** Those land on checks alone, so folding them only delays them behind the HARD approval.
- **A `clm sync` three-way merge instead of a fold.** It would make each fresh merge cheaper, but every land would still stale the rest and cost a fresh approval.

Refines `backlog-triage`.
