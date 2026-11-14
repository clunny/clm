---
status: applied
resolves:
  - a-draft-is-launched-for-a-code-only-bug-while-its-component-is
  - a-resumed-draft-s-first-command-fails-when-the-clone-still-holds
---

## Intent

A resumed draft's first command works in the clone that ran the draft's first session: `orchestrator.triage/draft-resume` now says the `git switch` command the prompt gives succeeds whether or not the clone still holds the local branch `spec/<id>`, and a test runs that command instead of reading it.

The other item, a draft launched for a code-only bug whose component a build holds, is accepted and deleted. Nothing is designed for it.

## Problem

**The resumed draft.** `clm draft --resume <id>` writes a prompt whose first command is `git switch -c spec/<id> --no-track origin/spec/<id>`. `-c` refuses a branch name that exists. The build loop resumes an orphaned draft in the clone that launched it (`orchestrator.loop`), and that clone holds the local branch `spec/<id>` the first session made, which outlives the session's worktree. So the command fails in the usual case of a resume, not in a rare one, and the session has to work out for itself what to do. `draft-resume` says the session "switches its checkout to that branch as `spec/<change-id>`", but its tests only read the command's text, so they pass while the command fails.

The design isn't wrong; the requirement didn't say which clone states the command must work in, and nothing ran it.

**The draft for a claimed component.** A draft judged a bug code-only, asked for `clm claim clm --fix`, and was refused because a build held `impl/clm`. It left the item, and a later draft did the same.

## Approaches

For the resumed draft:

- **The requirement says the command works with or without a local branch that no worktree holds, and a scenario runs it** (chosen). The build changes the prompt's command (`git switch -C spec/<id> --no-track <ref>` does it) and adds a test that makes the stale branch, takes the command from the prompt file and runs it in a new worktree. `origin`'s commit wins over the local branch's, which is what the requirement already says the session continues from.
- **`clm draft --resume` deletes the stale local branch before writing the prompt.** Rejected: the command would change the clone's branches, which `clm draft` otherwise never does, and the prompt's command would still fail in a clone the session runs in if the two differ.
- **Fix the prompt text on a code branch, with no spec change.** Rejected: the existing tests of `draft-resume` pass against the failing command, so nothing written proves the behaviour (`code-fix-needs-a-written-rule`), and which commit wins when the local branch differs is a choice a requirement should state. The component is pending anyway, so its queued build carries this at no extra land.

For the draft on a claimed component, accepted (see below). The fixes considered and not taken:

- **The draft fixes the bug on a branch of its own, with no claim, when the fix claim is refused.** It removes the wait, and `small-change-no-claim` already says two branches on the same files meet safely at `clm land`. But it is one more route onto `main`, and the plan in force collapses the eight there are to about three in its phase 5.
- **`clm triage` or `clm draft` holds back a bug whose components are all claimed.** Neither can tell a code-only bug from one that needs a spec change, which is the session's reading, so it would also hold back bugs a draft could design, for as long as builds hold the component.
- **A wait state recorded on the item.** A new key in the backlog format for a cost of one draft session.

## Accepted: a draft for a code-only bug whose component is claimed

`a-draft-is-launched-for-a-code-only-bug-while-its-component-is` is accepted as it stands. The plan in force (`backlog/a-passing-check-proves-less-than-the-spec-claims`, Settled) says defects found during the freeze are filed and their default answer is "accept", and this one gives no reason to depart from it:

- Nothing wrong reaches `main`. The draft's claim is refused, it leaves the item, and the item stays in the backlog for the draft whose claim wins.
- The cost is one draft session for each such bug in a triage or build session whose component a build holds at that moment. A session drafts an item only once, so it doesn't loop within one.
- Each fix adds a route onto `main` or a state to the backlog, in the part of the method phase 5 of the plan reshapes.

## Outcome

- `orchestrator.triage/draft-resume` gains one sentence and one scenario, *the first session's local branch is still there*. Its build changes the command in the resume prompt (`crates/clm-triage/src/draft.rs`) and the two tests in `crates/clm/tests/draft.rs` that assert the command's text, `scenario_a_draft_that_pushed` and `resume_takes_the_newer_of_the_two_spec_branches`, and adds the test that runs the command.
- `backlog/a-draft-is-launched-for-a-code-only-bug-while-its-component-is.md` is deleted as accepted, and `backlog/a-resumed-draft-s-first-command-fails-when-the-clone-still-holds.md` when the change is applied.

No capability test sees anything different at the product's boundary: the capability tests that reach `clm draft --resume` do so through the build loop and read no command from the prompt.

## Constraints

- `draft-resume` relies on no requirement that isn't current.
- `clm draft --resume` still creates no local branch and makes no worktree; only the command the session runs changes.

## Edge cases and workarounds

- A local `spec/<id>` holding commits `origin` lacks: the command moves the branch to `origin`'s commit, and those commits stay reachable only through the reflog. A draft pushes after each commit, and `autosync/spec/<id>` holds the snapshot of what it didn't push, which the resume already prefers when it is newer.
- A local `spec/<id>` that the first session's worktree still has checked out: git refuses a second checkout of the branch, and the requirement doesn't cover it. Not met so far; the session that meets it reports it.

## Open questions

None.
