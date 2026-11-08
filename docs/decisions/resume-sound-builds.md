---
title: "A stopped build whose work is sound is resumed, by the orchestrating session's judgement"
date: 2026-11-07
status: accepted
---

`resume-limited-builds` let `build` resume a build that a usage or rate limit cut off, with `SendMessage`, and built every other stopped build again with `clm run` and a new subagent. A session then had several builds stop only because the disk filled during `clm land`: their code was done and reviewed, and only the land had to run again once space was freed. The skill left the session no choice but to build them again, so each retry lost the stopped build's context (what it tried, the review findings it weighed), compiled the workspace again from an empty `target/` (several GB each, on a machine already short of disk), and lost the files the build hadn't committed, such as a drafted pull request body.

The limit was one case of a broader one: the work is sound and the stop came from outside it. So `build` judges, from why a build stopped, whether to resume that same subagent or build the component again:

- **Resume** a build whose work is sound and whose stop came from outside the work: a usage or rate limit, a full disk, a network outage, a tool or host failure that has passed. As before, it runs no `clm run --finish` for it, so the build keeps its place under `max_builds` and its worktree, and counts as still running. It tells the human why the build stopped and, when only they can clear the cause, what to clear; it resumes the subagent once the cause has cleared, telling it why it stopped, that the cause has cleared, and to run again any review or check subagent of its own that the stop killed.
- **Build again**, with the existing retry, a build that went wrong in its approach, ran out of context, or stopped again for the reason it was resumed for. A build is resumed at most once for the same reason: a cause that comes back is not as passing as it looked, and a fresh context is the next thing to try. That second stop is the retry's first, so the retry's "stops twice in a row for the same reason" still bounds the whole.
- A build that stopped blocked on a backlog item or a spec change is neither, as before.
- When the resume fails (`SendMessage` errors, or the subagent or its worktree is gone), it is a stop, and the retry applies.

The judgement is the session's, from the build's `stopped:` reason and what it knows of the machine, since the reasons are free text and the cases are open-ended; the skill states the criteria and names the common causes rather than listing the only ones allowed.

Supersedes `resume-limited-builds`, whose reasoning carries over: the place stays held because no `clm run --finish` runs, nothing in `clm` records a paused build, and the resume happens on the session's first turn after the cause clears, which may need the human to say so.

Rejected:
- **Keep the limit as the only resumable stop.** Throws away sound work's context and recompiles it for interruptions that weren't the build's, the case that prompted this.
- **Resume every stop.** A build that went wrong in its approach or ran out of context carries its mistakes or has no room left; a fresh build with `clm rebase` is the better next step.
- **A fixed list of resumable reasons in `clm`, matched against the `stopped:` line.** The reasons are prose the build writes, and new environmental failures keep turning up; a list would miss them or need a reason syntax the builds don't have (`yagni`).

Accepted edge cases: a cause the session misjudges as passing makes the resumed build stop again for it, and the retry takes over. A stopped build's worktree removed by something else before the resume (`clm wrap-up`, or `clm run` resuming another branch) makes `SendMessage` fail, which counts as a stop; keeping that worktree is `backlog/a-stopped-build-s-worktree-is-removed-before-the-build-can-be`'s.
