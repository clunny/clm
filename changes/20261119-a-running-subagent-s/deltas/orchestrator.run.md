---
component: orchestrator.run
---

## MODIFIED Requirements

### Requirement: run <!-- base: 5c35d326e5c50cfb -->
`clm run <component>` SHALL first remove leftovers of an earlier build of the component that are already landed: a worktree that has `impl/<component>` checked out, has no uncommitted changes or untracked files that aren't ignored, whose `HEAD` is an ancestor of `origin/main` and whose lock isn't held (`vcs/held-lock`), unlocking it first; and then a local branch `impl/<component>` that no worktree has checked out and whose tip is an ancestor of `origin/main` or is an unpushed claim: a commit with one parent, the same tree as that parent, the parent an ancestor of `origin/main`, on a branch that has never held `origin/impl/<component>`'s tip as below, or with no `origin/impl/<component>`. When the local branch `impl/<component>` then exists and isn't landed, it SHALL exit 1 naming the branch, without claiming, unless `origin/impl/<component>` exists and the local branch has held its tip, which is so when that tip is an ancestor of the local branch's tip or of one of the entries of the local branch's reflog, the test git's `--force-if-includes` makes, in which case it SHALL resume without claiming, first removing a worktree that has the branch checked out when that worktree has no uncommitted changes or untracked files that aren't ignored, unlocking it first. Otherwise it SHALL claim the component through `orchestrator`'s `claim_local`, so that the local branch `impl/<component>` is at the claim commit, tracking `origin/impl/<component>`, before the claim is pushed, exiting 1 when the claim is refused and 2 when it fails in git. When a worktree that has `impl/<component>` checked out has uncommitted changes or untracked files that aren't ignored, it SHALL exit 1 naming its path, without claiming. When a worktree that has `impl/<component>` checked out has none, its `HEAD` is an ancestor of `origin/main` and its lock is held, it SHALL keep it and exit 1 naming its path and saying a running process holds its lock, without claiming. It SHALL keep `.clm/` out of git through `.git/info/exclude`, create no worktree, and start no `claude` process.

It SHALL then write the prompt to `.clm/prompts/<component>.md` under the directory it runs in, replacing any earlier file, print on stdout only a launcher of at most five lines that names that file by its absolute path, says to read all of it before anything else and follow it, and names `git switch impl/<component>` as the file's first step, and exit 0. It SHALL create `.clm/prompts/` before claiming, refusing with exit 1 when it can't; when writing the file fails after the claim, it SHALL exit 1 naming the path. The prompt's first line SHALL say to run `git switch impl/<component>`. It SHALL name the skills `superpowers:writing-plans`, `superpowers:test-driven-development` and `superpowers:verification-before-completion`, each with the path `clm superpowers <skill>` prints; SHALL name the plan path `.clm/plans/<component>.md`; SHALL say when resuming a branch with unlanded commits that the branch exists; and SHALL end with the `clm context` bundle. When it exits 1 or 2, it SHALL print nothing on stdout.

#### Scenario: node prepared
- GIVEN a ready component `b`, `CLM_SUPERPOWERS_DIR` set to a directory holding the three skills, and a fake `claude` on PATH that records whether it ran
- WHEN `clm run b` runs
- THEN it exits 0, `origin/impl/b` exists, the local branch `impl/b` is at the claim and tracks it, no worktree has `impl/b` checked out, the fake never ran, stdout is at most five lines, names the absolute path of `.clm/prompts/b.md` and `git switch impl/b`, and doesn't contain `b`'s context bundle, and that file's first line names `git switch impl/b`, and the file has the three skill names each with its `SKILL.md` path under `CLM_SUPERPOWERS_DIR` and the plan path `.clm/plans/b.md`, and ends with `b`'s context bundle

#### Scenario: claimed elsewhere
- GIVEN `origin/impl/b` committed 1 hour ago, and no local branch `impl/b`
- WHEN `clm run b` runs
- THEN it exits 1 and prints nothing on stdout

#### Scenario: resume with unlanded commits
- GIVEN a clean worktree left by an earlier build with `impl/b` checked out, its branch pushed with a commit `origin/main` lacks
- WHEN `clm run b` runs
- THEN no claim commit is pushed, that worktree is gone, the local branch `impl/b` still has the commit, and the prompt it writes says the branch exists

#### Scenario: landed leftover worktree
- GIVEN a clean worktree with local branch `impl/b` checked out, left by an earlier build whose branch landed on `origin/main` with a merge commit, with `origin/impl/b` still at that tip, the worktree locked, and a ready component `b`
- WHEN `clm run b` runs
- THEN the worktree is gone, it pushes a new claim commit whose parent is `origin/main` to `origin/impl/b`, the local branch `impl/b` is at that commit, and it exits 0

#### Scenario: landed leftover branch
- GIVEN a local branch `impl/b` with no worktree, landed on `origin/main`, no `origin/impl/b`, and a ready component `b`
- WHEN `clm run b` runs
- THEN it claims `b`, the local branch `impl/b` is at the claim, and it exits 0

#### Scenario: unpushed leftover branch
- GIVEN a local branch `impl/b`, whose reflog never held `origin/impl/b`'s tip, with a commit that changes a file and that neither `origin/main` nor `origin/impl/b` has, where `origin/impl/b` is a lapsed claim, and a ready component `b`
- WHEN `clm run b` runs
- THEN it exits 1 naming `impl/b`, pushes no claim commit, prints nothing on stdout, and `impl/b` still has the commit

#### Scenario: stopped after a rebase
- GIVEN a ready component `b` whose claim this clone pushed, a clean, locked worktree at `.claude/worktrees/agent-1` with the local `impl/b` checked out, and the local `impl/b` then rebased onto a newer `origin/main`, so that it no longer contains `origin/impl/b`, and given a commit that changes a file, none of it pushed
- WHEN `clm run b` runs
- THEN it exits 0, pushes no claim commit, `origin/impl/b` is unchanged, that worktree is gone, the local `impl/b` still has the commit, and the prompt it writes says the branch exists; and `clm branch push`, run on that branch, then pushes it

#### Scenario: claim rebased before any work
- GIVEN a ready component `b` whose claim this clone pushed, the local `impl/b`, checked out nowhere, rebased onto a newer `origin/main` and holding nothing else, so that its tip is a commit with one parent, the parent's tree, and not contained in `origin/impl/b`
- WHEN `clm run b` runs
- THEN it exits 0, pushes no claim commit, `origin/impl/b` is unchanged, and the local `impl/b` is still that commit

#### Scenario: interrupted after the push
- GIVEN a ready component `b` whose claim this clone pushed 1 minute ago, with the local branch `impl/b` at that claim commit, checked out nowhere, and nothing else done
- WHEN `clm run b` runs
- THEN it exits 0, pushes no claim commit, the local `impl/b` and `origin/impl/b` are still that commit, and it writes `b`'s prompt

#### Scenario: interrupted before the push
- GIVEN a ready component `b`, no `origin/impl/b`, and a local branch `impl/b`, checked out nowhere, at a commit whose parent is `origin/main` and whose tree is `origin/main`'s
- WHEN `clm run b` runs
- THEN it exits 0, `origin/impl/b` is a new claim commit whose parent is `origin/main`, and the local `impl/b` is at it

#### Scenario: another clone claimed meanwhile
- GIVEN the local `impl/b` of "interrupted before the push", and `origin/impl/b` another clone's claim committed 1 hour ago
- WHEN `clm run b` runs
- THEN it exits 1, prints nothing on stdout, and `origin/impl/b` is unchanged

#### Scenario: landed leftover in use
- GIVEN the leftover worktree of "landed leftover worktree", locked with the reason `claude agent agent-1 (pid <p> start 1)`, `<p>` the ID of a process that is running
- WHEN `clm run b` runs
- THEN it exits 1 naming the worktree's path, pushes no claim commit, prints nothing on stdout, and the worktree is still there with `impl/b` checked out

#### Scenario: dirty leftover
- GIVEN the leftover worktree as in "landed leftover worktree", with an untracked file `notes.txt`
- WHEN `clm run b` runs
- THEN it exits 1 naming the worktree's path, pushes no claim commit, prints nothing on stdout, and `notes.txt` is still there

#### Scenario: node that writes a protected path
- GIVEN a ready component `w` whose `paths` are `["web/**"]` and whose pending requirement names `web/.npmrc`
- WHEN `clm next` and `clm run w` run
- THEN `clm next` prints `w`, and `clm run w` exits 0 and writes `w`'s prompt

#### Needs
- clm/renamed-commands
- orchestrator/claim-local
- vcs/held-lock
