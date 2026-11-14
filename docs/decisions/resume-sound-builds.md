---
title: "A stopped build whose work is sound is resumed, by the orchestrating session's judgement"
date: 2026-11-07
status: accepted
---

`build` first resumed, with `SendMessage`, only a build that a usage or rate limit cut off, and built every other stopped build again with `clm run` and a new subagent. A build cut off that way hasn't gone wrong: its claim, worktree and context are all intact, and building it again throws the context away and restarts from its last push. A session then had several builds stop only because the disk filled during `clm land`: their code was done and reviewed, and only the land had to run again once space was freed. The skill left the session no choice but to build them again, so each retry lost the stopped build's context (what it tried, the review findings it weighed), compiled the workspace again from an empty `target/` (several GB each, on a machine already short of disk), and lost the files the build hadn't committed, such as a drafted pull request body.

The limit was one case of a broader one: the work is sound and the stop came from outside it. So `build` judges, from why a build stopped, whether to resume that same subagent or build the component again:

- **Resume** a build whose work is sound and whose stop came from outside the work: a usage or rate limit, a full disk, a network outage, a tool or host failure that has passed. It runs no `clm run --finish` for it, so the build keeps its place under `max_builds` and its worktree, and counts as still running. It tells the human why the build stopped and, when only they can clear the cause, what to clear; it resumes the subagent once the cause has cleared, telling it why it stopped, that the cause has cleared, and to run again any review or check subagent of its own that the stop killed. The resumed build returns like any other and is finished then.
- **A limit or a full disk is resumed each time** it stops the build and clears. A cause that comes back from outside the work says nothing against the work: it says the cause isn't fixed, and a rebuild is no remedy, since a fresh build would meet the same disk or limit and only repeat the work. The bound on such a stop is the cause clearing, which for a full disk the human confirms. When the same outside cause stops builds again, `build` tells the human it recurred and acts on what brings it back where it can: when parallel builds are what fill the disk or reach the limit, it resumes the builds it stopped one at a time and launches no new build until they have returned.
- **Every other stop is resumed at most once for the same reason**: a stop whose cause could lie in the work itself, such as a check or test that keeps failing, and a network outage or a tool or host failure that had already passed when the build stopped, which the session resumes at once with nothing to wait for. With no reset and no word from the human between resumes, a cause misjudged as outside the work would otherwise be resumed for ever, and one that keeps coming back may be the work's.
- **Build again**, with the existing retry, a build that went wrong in its approach, ran out of context, or stopped again for a reason it is resumed for only once: a fresh context is the next thing to try. That second stop is the retry's first, so the retry's "stops twice in a row for the same reason" still bounds the whole.
- A build that stopped blocked on a backlog item or a spec change is neither.
- When the resume fails (`SendMessage` errors, or the subagent or its worktree is gone), it is a stop, and the retry applies.

The judgement is the session's, from the build's `stopped:` reason and what it knows of the machine, since the reasons are free text and the cases are open-ended; the skill states the criteria and names the common causes rather than listing the only ones allowed.

The place stays held because no `clm run --finish` runs, nothing in `clm` records a paused build, and the resume happens on the session's first turn after the cause clears, which may need the human to say so: the session that runs `build` is usually cut off by the same limit as its builds.

The each-time rule came from one session. A full disk stopped six builds; they were resumed once space was freed, and the disk filled again because their land checks ran in parallel. Resuming each at most once would have built all six again, on the session's own model, though their work was sound and a fresh build would meet the same disk. The human had them resumed one at a time, and all six landed.

Rejected:
- **Keep the limit as the only resumable stop.** Throws away sound work's context and recompiles it for interruptions that weren't the build's, the case that prompted this.
- **Resume every stop.** A build that went wrong in its approach or ran out of context carries its mistakes or has no room left; a fresh build with `clm rebase` is the better next step.
- **A fixed list of resumable reasons in `clm`, matched against the `stopped:` line.** The reasons are prose the build writes, and new environmental failures keep turning up; a list would miss them or need a reason syntax the builds don't have (`yagni`).
- **A count for outside stops** (resume one at most two or three times). Any count picks a number the cause doesn't respect, and every rebuild past it repeats sound work for nothing.
- **Run `clm run --finish` and then resume with `SendMessage`.** The finish frees the build's place, so the resumed build would run outside the clone's cap and another launch could take its place.
- **A `clm` command that records a build as paused.** Nothing in `clm` needs to know: the place is already held until `--finish`, and only the session that launched the subagent can resume it.

Accepted edge cases:
- A cause the session misjudges as passing makes the resumed build stop again for it. For a stop resumed at once, the retry then takes over; for a limit or a full disk, each resume waits for the reset or the human, who sees the recurrence.
- A stopped build's worktree removed by something else before the resume (`clm wrap-up`, or `clm run` resuming another branch) makes `SendMessage` fail, which counts as a stop; keeping that worktree is `backlog/a-stopped-build-s-worktree-is-removed-before-the-build-can-be`'s.
- A cause that outlasts the claim's four-hour lapse lets another session take the component in the meantime; the resumed build then meets that work when it rebases or pushes, and when it can't go on it stops, which the retry handles.
