---
title: "Health is one advisory report; a requirement's signals accumulate to a threshold before a question is filed"
date: 2026-10-28
status: accepted
---

The checks were scattered (`clm check`, `clm pending`, `clm size`, `clm ledger verify`, `clm status`), and nothing noticed when a requirement or a design was fighting its implementation. `deterministic-enforcement` lets model judgment advise but never gate, and `oversized-is-a-state` had already settled how a program handles a state a human must decide: show it, file a `question`, block nothing.

So `clm health` (node `check.health`) is one report of the repository's health, advisory throughout: it exits 0 whatever it finds, and no rule of `clm check`, `clm gate` or `clm land` reads it. It rolls up what the other commands show and adds what nothing showed: lapsed claims, stalled changes, unanswered questions, uncited decisions, history words in specs, Design-heavy nodes, size outliers, code no test exercises, and the per-requirement signals.

A requirement's signals are counted by a program from committed files alone (`git-is-truth`): its `friction` flags (`friction-flags`), the archived changes that modified it, and whether its scenarios or exception clauses exceed a setting. They add up to a score, but only once a build has flagged the requirement: without a flag the score is zero, whatever the archive shows, because the archive of this repository already holds about thirty requirements rewritten three or more times, and counting them alone would have filed thirty questions on the first builds after this landed. With a flag, the rewrites and counts weigh in, and when the score reaches the `reconsider` threshold `clm sync` on the node's `impl/*` branch files one `question` item, `reconsider-<node>-<slug>`. The human decides: drop the requirement, change it, or redesign around it, through a spec change. Thresholds are settings of the repository (`[health]` in `clm.toml`), so a project tunes them in a reviewed file; the weights are code.

Bloat (history words, a heavy Design, outliers, unexercised code) is reported, not filed: the remedy is judgment, which the `health` skill supplies through the `spec-slimmer` subagent as a proposal, and a human takes it into `/clm:propose` or not.

Rejected:
- **Signals as `clm check` violations.** The build that trips one can't fix it: the remedy is a spec change. Same reason `oversized-is-a-state` rejected it.
- **One flag files a question.** One session's judgment would fill the human's queue, the reason `defect-reports` rejected a Stop hook asking after every turn. Accumulation keeps the human's list short and each item evidenced.
- **Flags only in the report.** Nothing reaches `waiting on you` unless someone runs the report.
- **A skill that counts by reading.** Counting in prose drifts; the counting is code, the judgment is the skill.
- **Spreading the rules over `check`, `sync` and `plugin.hooks`.** All three are over budget, and health rules change for different reasons than the blocking checks, the seam `slim-nodes` splits on.

## The rule in full

`ARCHITECTURE.md` stated this as the entry `health-advises` until 2026-11-13, when it moved down here; its rules serve the capability `know-health` (`human-layer-entries-stay-short`). The text still holds:

`clm health` SHALL be advisory: it exits 0 whatever it finds, and no check, gate or land reads it.

**Scenario: unhealthy repository**
- GIVEN a repository with oversized components and workaround flags
- WHEN `clm health` runs
- THEN it reports them and exits 0
