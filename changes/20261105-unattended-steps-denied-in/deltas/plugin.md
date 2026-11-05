---
component: plugin
---

## ADDED Requirements

### Requirement: skills-push-with-clm
No `SKILL.md` under the plugin's `skills/` SHALL contain `git push`. Wherever a skill tells Claude to push a branch to `origin`, attended or not, it SHALL name `clm branch push`, and wherever it tells Claude to delete a branch on `origin`, `clm branch delete`, so that under Claude Code's auto mode every step that writes to `origin` is a `clm` command the project's allow rules cover (`unattended-steps-run-clm`).

#### Scenario: skills push through clm
- GIVEN the plugin's `skills/*/SKILL.md`
- WHEN a test reads each
- THEN none contains `git push`, and `propose`, `contribution`, `triage` and `build` each contain `clm branch push`

#### Scenario: skill with a raw push
- GIVEN a `SKILL.md` that tells Claude to push with `git push --force-with-lease`
- WHEN the test's check runs on it
- THEN it reports the skill and `git push`

#### Serves
- capability/build-from-spec
- capability/triage-unattended

#### Needs
- host/branch-push
- host/branch-delete

### Requirement: unattended-permissions-check
`skills/build/SKILL.md` and `skills/triage/SKILL.md` SHALL each tell Claude, once, before the first build or draft it launches, to run `clm init --check-permissions` and, when it exits 1, to tell the human that in auto mode Claude Code may deny the unattended steps those rules allow, show the rules it printed, say that they belong in `permissions.allow` in `.claude/settings.json` (or `.claude/settings.local.json` for this clone alone), and go on launching, never refusing to; when it exits with any other code, as a `clm` without the option does, to tell the human the check couldn't run and go on launching; and never to write either file itself. `skills/build/SKILL.md` SHALL tell Claude to push a branch it rebased with `clm branch push` and to release a claim it took with `clm branch delete`, and SHALL NOT contain `git push --force-with-lease`.

#### Scenario: skills check the rules
- GIVEN the plugin's `skills/build/SKILL.md` and `skills/triage/SKILL.md`
- WHEN a test reads them
- THEN each contains `clm init --check-permissions`, `auto mode` and `.claude/settings.json`, and the build skill contains `clm branch push` and `clm branch delete` and doesn't contain `git push --force-with-lease`

#### Scenario: skill without the check
- GIVEN a `triage` `SKILL.md` that doesn't mention `clm init --check-permissions`
- WHEN the test's check runs on it
- THEN it reports the skill and `clm init --check-permissions`

#### Serves
- capability/build-from-spec
- capability/triage-unattended

#### Needs
- init/init-check-permissions
- host/branch-push
- host/branch-delete

## MODIFIED Requirements

### Requirement: propose-elaborates-ideas <!-- base: c3cf5e73b1d1ebc7 -->
`skills/propose/SKILL.md` SHALL tell Claude, in a section of its own outside the numbered steps, that an idea may be elaborated over several sessions in its own backlog item (`elaborate-ideas-on-main`):
- **Start.** When the human wants to elaborate an `idea` item, Claude first looks for an unlanded branch of it on `origin` with `git ls-remote origin 'refs/heads/spec/*-idea-<slug>'` and resumes that one when there is one, in a worktree of its own; otherwise it claims the item with `clm claim backlog/<slug>` and stops when the claim is refused, as for a change, and cuts `spec/<YYYYMMDD>-idea-<slug>` from `origin/main` in a worktree of its own. A new idea is cut the same way without a claim, its item written with `title`, `kind: idea` and `components` in its frontmatter.
- **Elaborate.** The branch holds no `change.md` and no delta. Claude keeps in the item's body the sections `## Intent`, `## Settled` (each choice made, with its reason), `## Approaches` (the options still open) and `## Open questions` (what the next session takes up), rewriting them as the brainstorm settles things, starting from the item's `## Open questions` when it has them, and commits and pushes as it goes, never landing at every pause.
- **Land once.** At the end of the session, or when the human asks for a checkpoint, Claude runs `clm rebase` until its last line is `rebase ok`, pushes, and lands the branch with `clm land`, landing by the first line of `clm gate --base origin/main` (`spec`, or `code` for a branch that only adds a new item, both landing on checks). On `landed <sha>` it releases the claim it took and the branch's autosync snapshot, deleting `propose/<slug>` and `autosync/spec/<YYYYMMDD>-idea-<slug>` on `origin`, each with `clm branch delete <ref>`, since nothing else deletes them while the item stays in the backlog, and then runs `clm wrap-up`. Work after a land in the same session goes on a branch cut afresh from `origin/main`. A session that runs past the claim's expiry claims the item again with `clm claim backlog/<slug>` once the claim has lapsed.
- **Propose.** Once the human agrees to the whole design, the change is proposed as any other, with `from: backlog/<slug>`, its `change.md` seeded from the item's sections and the item deleted on the spec branch.

#### Scenario: skill elaborates ideas
- GIVEN the plugin's `skills/propose/SKILL.md`
- WHEN a test reads it
- THEN it contains `## Settled`, `## Open questions`, `-idea-` and `clm wrap-up`

#### Scenario: skill without elaboration
- GIVEN a `propose` `SKILL.md` that doesn't mention `## Settled`
- WHEN the test's check runs on it
- THEN it reports `## Settled`

#### Needs
- host/branch-delete
- clm/renamed-commands

#### Serves
- capability/propose-intent
- capability/keep-a-backlog

### Requirement: triage-revises-declined-fold <!-- base: 94dd9989f33dbc71 -->
`skills/triage/SKILL.md` SHALL tell Claude, in the step that asks the human to approve a fold's intent, that on a no that comes with a different design (an answer the human gives in the session about what a folded change should do instead) it revises the fold as `skills/propose/SKILL.md`'s step 12 revises a declined change: it discusses what to change, switches into the worktree that holds `spec/<batch>` as `triage-enters-branch-worktree` says for work on the branch, edits there each folded change the human's answer concerns, re-applies it with `clm rebase` until its last line is `rebase ok`, pushes with `clm branch push`, asks again, and on a yes approves and lands it as on a first yes. It SHALL say that this is the one design the session does in its own context, since the human is in the conversation and `clm draft` can't take the item again while the fold's branch still proposes it, and that on a plain no, with no different design, it leaves the branch as before.

#### Scenario: skill revises a declined fold
- GIVEN the plugin's `skills/triage/SKILL.md`
- WHEN a test reads it
- THEN its step that runs `clm approve spec/<batch>` contains `different design`, `clm rebase`, `clm branch push` and `triage-revises-declined-fold`

#### Scenario: skill that only leaves the branch
- GIVEN a `triage` `SKILL.md` whose approval step doesn't mention `triage-revises-declined-fold`
- WHEN the test's check runs on it
- THEN it reports `triage-revises-declined-fold`

#### Serves
- capability/triage-unattended

#### Needs
- host/branch-push
