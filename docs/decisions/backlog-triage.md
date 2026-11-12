---
title: "The backlog is worked down unattended: deterministic groups, drafts in subagents, landing by the gate, and one approval list for the human"
date: 2026-10-26
status: accepted
---

A backlog of a hundred items, almost all `P5`, costs the human more in deciding what to propose next and in sitting through each `propose` session than in the reviews `humans-review-intent` reserves for them. Most bug items are stale text in a Design or a skill, or code that falls short of a requirement it has: AUTO edits or code-only fixes, which no human reviews, but nothing told them apart from the changes that need one.

So the backlog is triaged the way nodes are built (`builds-in-subagents`). `clm triage` groups the untaken items by a fixed rule, by kind class (`bug`, or `idea` and `deferred` together) and exact node set, cut to a budget, and prints each group with its priority and the requirements its items' `blocks` would release, beside the proposed spec work already on `origin` and the items waiting on the human. `clm draft <slug>...` claims a group (`propose-claims`) and prints the prompt for an unattended proposing session, as `clm run` prints a build's. The `triage` skill runs one draft per group in a background worktree subagent, in parallel. A draft designs one change, applies it, runs the gate and the builder check, and lands itself when the tier is `AUTO` or `SOFT`; a `HARD` one stops on its branch. Bugs that turn out code-only are fixed on `--fix` branches instead (`code-fix-resolves-bug`), so a draft's spec PR is never `mixed`. When the drafts return, the skill asks the human for each `HARD` change its own drafts left, in the session, in `clm triage`'s order, through the guard (`approve-in-session`); spec work already on `origin` is listed, never put up for approval, since its tier and its author are unknown to the skill.

What stays with the human is exactly what the rules reserve for them: the `HARD` approvals, the SOFT reviews `clm review` lists, the `question` and `human` items, and priorities. What moves is who starts the work. The grouping and the drafts are advisory; whether a branch lands alone is `clm gate`'s verdict on its diff (`computed-gate-tiers`, `deterministic-enforcement`), and a bad group costs a wasted subagent, never a wrong land.

Drafts skip brainstorming: it is a conversation (`design-brainstorming`), and the draft has none. The builder check is their review. An item a draft can't design without the human is left out of `resolves` and reported with the question; its claim lapses.

Rejected:
- **A graph of proposed changes, individually approvable and ordered by what each unblocks.** An approval is pinned to a diff's fingerprint (`pinned-approval`), so individually approvable units are separate branches anyway, and ordering between changes was removed on purpose (`node-reconciliation`): edges between proposed changes would bring back the waits that stalled builds. What was wanted from it, a sorted list of what to approve first, is derived from `blocks`, `resolves` and the queue.
- **Batch mode inside `propose`.** One PR per session with the human steering each; the backlog shrinks at their pace.
- **`clm triage` alone.** Honest about being advisory, but it starts nothing; it is kept as the first step.
- **Retrying a stopped draft from the skill.** A draft that pushed its branch holds its items; the next `clm triage` lists it as `open`, and `/clm:propose` resumes it. A resume flag can come later.

(Amended 2026-10-27, `20261027-triage-one-spec-branch`: the human is no longer asked for each `HARD` change separately. A `HARD` draft opens no page, and one fold subagent merges every `HARD` change the run's drafts left into one spec branch with one page, which the human approves once (`triage-folds-hard-drafts`).)

(Amended 2026-10-30, `20261030-clm-draft-s-prompt`: `deferred` items no longer form `design` groups. A deferred item waits on a condition its body names that only the human can judge, so every draft of one ended `stopped` and the next run drafted it again. A draft that meets an `idea` item waiting on such a condition sets its kind to `deferred`, so the next run leaves it; the human takes it up with `/clm:propose`. A draft whose branch edits only the backlog, deferring an item, correcting a bug's `nodes` or reporting a CLM defect, lands that edit as a change with no delta rather than deleting its branch.)

(Amended 2026-10-30, `20261030-triage-menu`: `idea` items no longer form `design` groups; `clm triage` lists each as an `idea` line that no draft takes, and the `triage` skill brainstorms it with the human in its session. The skill works only what the human picks from a menu (`ideas-brainstormed`).)

(Amended 2026-11-12, `20261112-build-drafts-bugs`: `/clm:build` also drafts the bug groups, each round of its drain, as triage drafts a picked `group` line; drafts that need review stay for the next `/clm:triage` to approve (`build-drafts-bugs`).)
