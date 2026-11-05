---
title: "Sessions sharing one checkout can't interfere, and the guide says how to run several"
kind: idea
components: [plugin, plugin.hooks, orchestrator.triage]
---

## Intent

A human may run several Claude Code sessions on one clone at once, such as `/clm:build` in one and `/clm:propose` in another. `session-worktrees` makes that safe for the branches the skills cut: each one gets a worktree of its own. The requirement (`plugin/session-worktrees`) is proven only by tests that read the skills' text, though, and it rests on two assumptions that nothing checks:

1. **A folder under `.claude/worktrees/` that the skills didn't add belongs to one session.** A human can start a session in such a folder (one made with Claude Code's worktree option, for instance) and then start a second session there. Both treat the folder as their own isolated checkout and `git switch` it onto their branches. That is the `HEAD`-moving collision that `session-worktrees` was decided to prevent, back again, and nothing detects it.
2. **Only one session in a checkout uses the files that `/clm:triage` and `/clm:build` keep there.** Each triage run deletes `.clm/answers.md`, then appends the human's answers to it and passes it to every draft. A second triage in the same checkout wipes the first one's answers, or hands them to its own drafts. The build skill's fix-reported-defects step "first removes a leftover `.claude/worktrees/defects`" and can't tell a leftover from another session's live one; only git's refusal to remove a dirty worktree protects it. Archiving already defers to another session's unlanded `impl/archive/<id>`.

`session-worktrees` rejected a runtime check because "git doesn't record which session uses a checkout". A record kept outside git, local to the clone like the build places (`build-places`), would make one possible.

The idea:

- **Enforce it.** Keep a per-checkout session record (for example, a file under the checkout's `.clm/` that the `SessionStart` hook writes with the session's ID, and that a stale-record rule like the build places' clears). A skill that changes the checkout or its `.clm/` state (`propose`, `triage`, and `build`'s archive and defect steps) refuses, naming the other session, when another live session holds the checkout. Prove it with a test that runs two simulated sessions, not only one that reads the skills' text.
- **Make per-session state per session.** Name triage's answers file and the build skill's `defects` and `archive` worktrees by session, so two sessions never share them even when the record is lost.
- **Recommend a practice in the guide.** Add a section to `docs/guide/teams.md` (or `using-clm.md`): start every interactive session from the clone's main checkout, never from a folder under `.claude/worktrees/` that another session uses; any number of `/clm:propose`, `/clm:health` and `/clm:help` sessions can run at once; run one `/clm:triage` and one `/clm:build` per clone at a time; and `max_builds` in `clm.local.toml` counts builds and drafts from every session on the clone.

## Open questions

- Is the record a lock that refuses, or a warning the skills act on? `deterministic-enforcement` argues for refusing, but a session that crashed must not hold the checkout forever.
- Does the guard (`plugin.hooks.guard`) enforce it on `git switch` and `git worktree remove` in a held checkout, or do the skills check it themselves?
- Should two `/clm:build` sessions on one clone be refused outright, or are claims and build places enough to let them share the queue?
- Is the guide practice a requirement on the page (like `guide-entry-pages-for-newcomers`), so a test keeps it there?
