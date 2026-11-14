---
title: "The backlog is worked down unattended: deterministic groups, drafts in subagents, landing by the gate, and one approval list for the human"
date: 2026-10-26
status: accepted
---

A backlog of a hundred items, almost all `P5`, costs the human more in deciding what to propose next and in sitting through each `propose` session than in the reviews `humans-review-intent` reserves for them. Most bug items are stale text in a Design or a skill, or code that falls short of a requirement it has: spec edits that land on checks or code-only fixes, which no human reviews, but nothing told them apart from the changes that need one.

So the backlog is triaged the way components are built (`builds-in-subagents`). `clm triage` groups the untaken `bug` items by a fixed rule, by exact component set, cut to a budget, and prints each group with its priority and the requirements its items' `blocks` would release, beside the proposed spec work already on `origin` and the items waiting on the human. Only bugs form groups:
- An `idea` item is listed as an `idea` line that no draft takes. Whether it is worth doing, and in what shape, is the human's call, so the `triage` skill brainstorms it with the human in its session (`ideas-brainstormed`).
- A `deferred` item is listed as a `deferred` line. It waits on a condition its body names that only the human can judge, so a draft of one would only stop and the next run would draft it again; the skill shows it to the human, who takes it up with `/clm:propose` or has it drafted. A draft that meets an `idea` item waiting on such a condition sets its kind to `deferred`, so the next run leaves it.

The skill works only what the human picks from a menu (`ideas-brainstormed`). `clm draft <slug>...` claims a group (`propose-claims`) and writes the prompt for an unattended proposing session, as `clm run` writes a build's (`build-prompt-file`). The `triage` skill runs one draft per picked group in a background worktree subagent, in parallel. A draft designs one change, applies it, runs the gate and the builder check, and lands itself when the gate's first line is `spec`; one that needs review (`spec review`) opens no page and stops on its branch. Bugs that turn out code-only are fixed on `--fix` branches instead (`code-fix-resolves-bug`), so a draft's spec branch is never `mixed`. A draft whose branch edits only the backlog (deferring an item, correcting a bug's `components` or reporting a CLM defect) lands that edit as a change with no delta rather than deleting its branch. When the drafts return, one fold subagent merges every change that needs review the run's drafts left into one spec branch with one page, and the skill asks the human for it once, in the session, through the guard (`approval-policy`, `yes-no-review`). Proposed spec work already on `origin` whose design is finished is offered for approval from the menu; work still being designed is only listed (`ideas-brainstormed`).

`/clm:build` also drafts the bug groups, each round of its drain, as triage drafts a picked `group` line, and folds the drafts that need review into one spec branch for one approval, as triage does (`build-drafts-bugs`, `build-loop-in-clm`).

What stays with the human is exactly what the rules reserve for them: the approvals of changes that need review, the `question` and `human-task` items, the ideas and deferred conditions, and priorities. What moves is who starts the work. The grouping and the drafts are advisory; whether a branch lands on checks is `clm gate`'s verdict on its diff (`yes-no-review`, `deterministic-enforcement`), and a bad group costs a wasted subagent, never a wrong land.

Drafts skip brainstorming: it is a conversation (`design-brainstorming`), and the draft has none. The builder check is their review. An item a draft can't design without the human is left out of `resolves` and reported with the question; its claim lapses.

Rejected:
- **A graph of proposed changes, individually approvable and ordered by what each unblocks.** An approval is pinned to a diff's fingerprint (`approve-pins-the-review`), so individually approvable units are separate branches anyway, and ordering between changes was removed on purpose (`node-reconciliation`): edges between proposed changes would bring back the waits that stalled builds. What was wanted from it, a sorted list of what to approve first, is derived from `blocks`, `resolves` and the queue.
- **Batch mode inside `propose`.** One PR per session with the human steering each; the backlog shrinks at their pace.
- **`clm triage` alone.** Honest about being advisory, but it starts nothing; it is kept as the first step.
- **Retrying a stopped draft from the skill.** A draft that pushed its branch holds its items; the next `clm triage` lists it as `open`, and `/clm:propose` resumes it. A resume flag can come later.
- **Asking the human for each change that needs review separately.** One run left many such branches that went stale against each other as each landed, each needing a merge and a fresh approval; the fold gives one branch and one approval.
