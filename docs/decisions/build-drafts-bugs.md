---
title: "/clm:build drafts the backlog's bug groups in its drain"
date: 2026-11-12
status: accepted
---

`/clm:triage` drafts a bug group only when the human picks *Bugs* from its menu, and that pick was in practice the only thing its bug path asked of them: a draft designs, fixes and lands on its own, and the rare one that needs review is approved later. Running `/clm:build` is already a yes to unattended work, so `build` drafts the bug groups itself, each round before its builds, the same way `triage` does: `clm draft <slug>...` per `group` line, a background worktree subagent on the session's own model, sharing the clone's build places (`build-places`).

What stays with the human is unchanged (`humans-review-intent`): a draft whose gate prints `spec review` stays on its branch, `clm triage` lists it as `designed`, and the next `/clm:triage` puts it up for approval. `build` asks for no approval mid-drain and takes no `human`, `idea`, `deferred` or `workaround` line, which need the human (`triage-owns-backlog-work`). An item a draft of the session took is never drafted again in that session, so an item a draft leaves can't loop.

`build`'s own step that fixed the defects its builds reported goes (`no-dead-code`): those defects are `kind: bug` items, and once a build lands they are on `origin/main`, where the next round drafts them. `propose` keeps its step, since a propose session doesn't drain the backlog.

There is no setting to turn it off: running `/clm:build` is the consent, and a project that wants a human to read every bug first runs `/clm:triage`, whose drafts are the same.

Amends `backlog-triage`: bug drafts start from `/clm:build` as well as from `/clm:triage`'s menu.

Rejected:
- **`clm next` prints draft lines beside components.** One deterministic queue, but it changes `orchestrator/next`'s contract, which `next-wait`, `queue-matches-next` and the `machine` runner rely on, and would have headless runners design changes, which needs a decision of its own. Worth revisiting once this has run for a while.
- **`build` offers the bugs at the end.** Still one question every run, which is what the change removes.
- **A setting, on by default.** It would add a key and a guide section for a choice `/clm:triage` already gives.
