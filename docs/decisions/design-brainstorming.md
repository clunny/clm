---
title: "propose brainstorms with superpowers' skill read by path; task sessions plan with writing-plans"
date: 2026-10-22
status: accepted
---

Refines `superpowers-for-execution` and `unattended-superpowers`.

**Design.** Superpowers' `brainstorming` skill fills a gap in `propose`: asking one question at a time, laying out approaches, and agreeing on one before writing. Three ways to make it available were considered:
- **(a) Read the pinned skill by path. Chosen.** `clm superpowers <skill>` prints the path of the pinned copy's `SKILL.md`, and `propose` reads and follows it.
- **(b)** A `clm design` launcher that starts an interactive session with the pinned plugin loaded.
- **(c)** Re-enable superpowers for interactive sessions.

(c) brings back the session-wide "brainstorm first" behaviour that was turned off, and (b) brings it back inside design sessions. (a) runs the skill only when `propose` asks for it. It uses the same pin, cache and eval process as `clm run`, so upstream improvements still arrive (`reuse-first`).

The skill's outputs map onto CLM's, never superpowers' own paths:
- its design doc becomes the `change.md` body, with its Approaches part as `## Approaches`;
- its significant choices become `docs/decisions/`;
- its approval step becomes the human's agreement in the conversation, and the spec PR stays the gate (`humans-review-intent`);
- its builder check is `propose`'s, run once before the PR;
- its `writing-plans` hand-off is replaced by the rest of `propose`.

The skill's own first step decides whether an intent needs the conversation, so `propose` keeps no separate "settled intent" rule.

**Planning.** Task sessions name `superpowers:writing-plans` alongside the TDD and verification skills. The plan is a session's working notes, not shared state, so it goes to `.clm/plans/` in the worktree, outside git, and a summary goes in the PR body. `changes/<id>/plan.md` stays the only shared plan (`git-is-truth`), and no committed plan doc goes stale (`spec-is-current`). `writing-plans` makes every plan require `executing-plans` or `subagent-driven-development`. Those chain into a new worktree, scripts that `dontAsk` would deny, and `finishing-a-development-branch`'s merge question. So the prompt has the session execute the plan itself and names those skills as ones not to invoke. `clm run` already provides the worktree, the review and the PR.

The skill list is a constant like the pin, and changes under the same rule: only once the `superpowers-unattended` skills arm passes with it. A test keeps that arm's prompt naming the same skills as the launch. That arm passed with planning on 2026-10-22 (see the change `20261022-design-brainstorming`). The fixture is too small to show that planning pays off, so that is judged from real task PRs. If it doesn't, the list shrinks again by the same route.
