---
status: applied
human_burdens:
  - Each `/clm:build` run also spends model tokens drafting the backlog's bug groups, on the session's own model.
---

## Intent

`/clm:build` drafts the backlog's bug groups itself, as part of its drain, the way `/clm:triage` drafts them when the human picks *Bugs*: each `group` line `clm triage` prints goes to `clm draft` and a background subagent, sharing the clone's build places with the builds. A draft that lands on checks lands; those that need review are folded into one spec branch at the end of the run, and the human is asked once to approve it. This replaces `build`'s own step that fixes the defects its builds reported, since those defects are bug items the next round drafts.

## Problem

The only human input in triage's bug path is the pick of *Bugs* from the menu and, for the rare draft that needs review, the approval. The pick is a yes to spend tokens on unattended work, which running `/clm:build` already is. So bugs wait in the backlog for a triage run that asks the human nothing they need to decide, while `build`, which is draining the queue anyway, leaves them alone, and carries a long procedure of its own (`fix reported defects`) for the one kind of bug it does touch: the defects its builds filed.

## Approaches

1. **`build` drafts bug groups in its own loop** (chosen). Each round it runs `clm triage` beside `clm next` and drafts every `group` line, as triage's step 5 does, before launching builds. A bug's spec fix can make its component pending, which the same drain then builds. Drafts that need review are folded with `clm draft --fold` once the drain is done, and the human approves the fold once, as in triage, so `build` never asks mid-drain. Skill text only; no CLI change.
2. **One queue: `clm next` prints draft lines too.** One deterministic ordering of builds and drafts, but it changes `orchestrator/next`'s contract, which `next-wait`, `queue-matches-next` and the `machine` runner rely on, and would have headless runners designing changes, which needs a decision of its own. Worth revisiting once (1) has run for a while.
3. **`build` offers the bugs at the end.** Smallest, but still a question every run, which is what the intent removes.

No setting turns it off (the human's call): running `/clm:build` is the consent, and a project that wants a human to read every bug first runs `/clm:triage`, whose drafts are the same.

## Outcome

- `build`'s loop: drafts first, in `clm triage`'s order, while a place is free, then builds for what `clm next` prints; repeat until `clm next` prints nothing, `clm triage` prints no `group` line whose items no draft of this session has taken, and none of its own builds or drafts is left.
- Once the drain is done, the drafts left for review are folded with `clm draft --fold` and the human is asked once for the fold's approval, as triage's steps 8 and 9 do. Left for `/clm:triage` one by one instead, they would go stale against each other as each landed (`triage-folds-hard-drafts`), and a fold branch left unapproved shows as no `designed` line.
- A draft's return is handled as triage's step 7 says: `clm draft --finish <id>` after every return, `landed <sha>` noted, `proposed <id> review` kept for the summary, `stopped:` never retried, `left` lines kept, `waiting:` holding the draft as still running.
- An item a draft of this session took is never drafted again in the session, so an item a draft leaves (`left <slug>: ...`, its claim released) doesn't loop.
- `build`'s **fix reported defects** step, its `.claude/worktrees/defects` worktree and the `build-defect-fixes-run-setup` rule go. `propose` keeps its own step: a propose session doesn't drain the backlog.

## Open questions

- The session-start hook's defect rule (`plugin.hooks`) still tells every session, build sessions included, to fix its reported defects on a `--fix` branch after landing. For a build session that is now redundant, since the drain drafts them, though not wrong; this change leaves the hook text alone.

## Constraints

- `humans-review-intent`: a draft that needs review lands only through the fold's approval, asked in the session through the guard.
- `build-places`: a draft holds one of the clone's places until `clm draft --finish`, as in triage.
- `model-per-launch`: a draft designs, so it launches on the session's own model, as in triage.
