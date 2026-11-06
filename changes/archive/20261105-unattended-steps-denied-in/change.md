---
status: done
resolves: [unattended-steps-denied-in-auto-mode]
human_burdens:
  - "Once the change is built, one check in notespeed in auto mode, from a worktree session, that a rebased build pushes with `clm branch push` and lands, and a no-review draft lands (backlog/check-subagent-honours-allow-rule); nothing waits on it."
  - "Each repository set up before this change needs its `.claude/settings.json` allow rules added by hand once (`clm init --check-permissions` names them); this repository's own are added by its root build, which Claude Code asks you to confirm as a protected write."
---

# Unattended steps run only clm commands, under allow rules clm init seeds

## Intent

Builds and drafts run alone in Claude Code's auto mode, as the `build` and `triage` skills promise. Today the auto-mode classifier denies a build's force-push of its rebased branch, a release of a claim by deleting its remote branch, and a draft's `clm land` of its own change, and the human has to run each by hand (issues #109, #154, #126). After this change every unattended step that writes to `origin` is a `clm` command: `clm branch push` pushes the checked-out branch with a lease on what the clone last saw, `clm branch delete` deletes a remote branch with the same lease, and `clm init` seeds narrow allow rules for the `clm` commands the prompts run into `.claude/settings.json`, so they resolve before the classifier. A repository set up earlier gets its missing rules named by `clm init --check-permissions`, which the `build` and `triage` skills run before their first launch. The attended skills push the same way: auto mode judges an attended session's force-push as it judges a build's, so no skill, and not this repository's `CLAUDE.md`, names `git push` (`plugin/skills-push-with-clm`, `clm/claude-md-pushes-with-clm`).

## Problem

- **Builds.** After `clm rebase` rewrites `impl/<component>` onto a newer `main`, the build prompt has the session run `git push --force-with-lease` before `clm pr open` and `clm land`. Auto mode's classifier denies it as destructive, and the build stops with a checked branch it can't land. Releasing an orphaned claim by deleting its remote branch is denied the same way.
- **Drafts.** When `clm gate` prints `spec`, the draft lands its own change with `clm land spec/<id>`. The classifier denies it as a merge without review.
- No allow rule exists for either, and `builds-in-subagents` chose to ship no allowlist, so the classifier decides every unattended step.

## Approaches

The human chose the design in triage on 2026-11-05 (the item's text): unattended steps run only `clm` commands, and `clm init` seeds narrow allow rules for them.

1. **An allow rule for `git push --force-with-lease`.** Rejected: it lets a session force-push any branch, another session's claim or spec branch included.
2. **The draft hands its change to the triage session to land.** Rejected: the main session meets the same classifier, so the step only moves.
3. **`clm` commands for every write to `origin`, under narrow allow rules (chosen).** A new `clm branch push`, beside the existing `clm branch delete` on the host adapter's `branch` command, pushes only the checked-out branch, leasing on the remote-tracking tip and, with `--force-if-includes`, only when the branch has held it, so it keeps the claim's compare-and-swap (`lease-claims`); both refuse `main` and `clm/` themselves (`guard-blocks-push-to-main`), since the guard never sees the `git push` inside them. `clm land` is safe to allow because it is the gate: it refuses a `spec review` branch without approval. `clm approve` is never allowed.

Where the push lives was weighed: `clm rebase` is also run by hand where no push is wanted, and `clm land` never pushes the branch, so a push of its own, beside `branch delete`, keeps each command doing one thing (`reuse-first`: the `branch` command and its lease already exist).

## Outcome

- `host`: `clm branch push`; `clm branch delete`'s `--lease` defaults to the tip last fetched and it deletes only a tip this clone pushed; both refuse `main` and `clm/` branches.
- `ARCHITECTURE.md`: `guard-blocks-push-to-main` says a `clm` command that pushes for a session refuses `main` itself, enforced by those two rules.
- `init`: on its first run `clm init` adds twenty rules, `Bash(clm <subcommand>)` and `Bash(clm <subcommand> *)` for `apply`, `branch`, `check`, `claim`, `defect`, `gate`, `land`, `pr`, `rebase` and `workaround`, to `.claude/settings.json`, creating or merging into it, and the root component it writes names that file in its `paths`; a rerun leaves the file alone (`init-rerun`) and names missing rules in its next steps; `clm init --check-permissions` prints the missing rules and exits 1.
- `orchestrator.run` and `orchestrator.triage`: the build, draft and fold prompts push with `clm branch push`, delete with `clm branch delete`, and never name `git push`.
- `plugin`: the `build` and `triage` skills run `clm init --check-permissions` before their first launch and tell the human what to add; the build skill pushes and releases claims through `clm branch`.
- `clm` (root): this repository's `.claude/settings.json` holds the seeded rules.
- The guide's build section says what auto mode needs.

## Constraints

- `redesign-before-workaround`: the design is wrong where unattended steps need commands the classifier denies; the fix moves those steps behind `clm` commands rather than widening rules.
- `approval-policy`, `humans-review-intent`: `clm approve` is never allowed, and `clm land` still refuses a branch needing review without approval.
- `init-rerun`: a rerun changes no existing file, so earlier repositories are told, not rewritten.
- `guard-blocks-push-to-main`, `land-locally`: `clm branch push` and `clm branch delete` refuse `main` themselves.

## Edge cases and workarounds

- A rule in the older `Bash(clm land:*)` form counts as missing for `--check-permissions`, which compares exact strings, so the check names exactly what it wants added.
- A CLM repository in a subdirectory gets its `.claude/settings.json` in that subdirectory, beside `clm.toml`; a session started at the git repository's top reads the top's settings instead (see Open questions).
- A `.claude/settings.json` that isn't JSON of the expected shape is left alone, and the next steps name the rules.

## The human's answers

- "can we do that one-time experiment now in a subagent before we file and approve this?"
- "interestingly I've never hit this issue while developing CLM in this project, but it occurs in the notespeed project (../notespeed)"
- "but we create worktree sessions on the fly ad hoc a lot so that wouldn't be good UX. Doesn't the worktree inherit the main claude settings? What if we put it in claude.settings.local or something in the main worktree?"
- "it still failed, even though the permission was listed"
- Asked whether to revise on these lines: (1) the decision records that a Bash allow rule doesn't cover the classifier's review of a subagent's task, so unattended prompts name only `clm` commands; (2) the experiment item becomes a non-blocking check after the build, in notespeed from a worktree session; (3) the four rule-seeding requirements are unblocked. Answer: "Revise as described".
- "why not have attended steps also use the clm command (like propose, contribution, triage)". Asked whether to extend the change so the attended skills (`propose`, `contribution`, triage's fold revision, `build`'s own rebase handling) and this repository's `CLAUDE.md` push with `clm branch push`, with a plugin requirement that no skill names `git push`. Answer: "Yes, revise the fold".

## Open questions

- **The premise, and what a test showed.** That a worktree-isolated background subagent's Bash calls are covered by the project's allow rule rests on Claude Code's docs. A test on 2026-11-05 found something the design must hold to: a Bash allow rule does not cover the classifier's review of a subagent's *task*, which refused a launch whose task spelled out a force-push, with the rule loaded. So unattended prompts and their launchers name only `clm` commands, never a raw destructive git step (see the decision). The rule-covers-the-call half can't be tested before this change is built, so `backlog/check-subagent-honours-allow-rule` is a check the human runs after the build, blocking nothing.
- **Subdirectory CLM repositories.** The rules go beside `clm.toml`. Claude Code reads project settings from where the session starts; a project that starts sessions at its git top while CLM lives in `tool/` would need the rules at the top. Left as is, since `clm init` writes every file of its own in the CLM repository.
- **Pushes inside `clm`.** `clm claim`, `clm run`, `clm draft --finish` and `clm wrap-up` run their own leased `git push` as subprocesses, which the classifier never sees, so they stay as they are.
- **How the rules match.** They match a command that starts with the bare name `clm`, as every prompt and skill writes it. A session that runs `clm` by another path (this repository's `CLM_BIN` setup puts `clm` on `PATH` the same way) matches only if it calls it `clm`.
- **Audit and builder check.** The architecture audit found that `Bash(clm branch *)` let a session delete another session's live claim: `clm branch delete` now deletes only a tip this clone pushed, `guard-blocks-push-to-main` now names `host/branch-push` and `host/branch-delete` and says such commands refuse `main` themselves, and `builds-in-subagents` points at the new decision. The builder check's questions on output, formatting, exit codes and step order are settled in the deltas.
