---
status: applied
resolves: [git-commands-fail-while-another-run-adds-a-worktree]
---

## Intent

Make `clm-git`'s `worktrees` list again after a failure, as `fetch` fetches again, so a command that lists a clone's worktrees while another run of the same clone adds or removes one goes on instead of failing: `vcs/worktrees-tries-again`.

## Problem

While `git worktree add` writes a new entry under `.git/worktrees/<name>/` (its `gitdir` written, its `commondir` still empty), and while `git worktree remove` deletes one, every git command that walks all worktrees dies on it with `fatal: failed to read .git/worktrees/<name>/commondir: Success`. Builds, drafts, lands and verify runs add and remove worktrees of one clone all the time. `clm-git`'s `fetch` already survives the race by running again (`fetch-tries-again`); `worktrees` runs `git worktree list` once, and every land's check (`clm-land`'s `runner::places_of`), `clm-run`, `clm-triage`, `clm-checkpoint`, `clm-orchestrator`, `clm-effort`, `clm-loop` and `clm-machine` call it. No requirement states that a listing holds against the race, so the fix needs one (`code-fix-needs-a-written-rule`). `clm-verify`'s `tree::places_of` already stopped listing worktrees for the same race (`b94f4a2e`, on `impl/verify`).

## Approaches

- **`worktrees` lists again after a failure, through the loop `fetch` already runs.** Taken. The race is git's and short-lived, `worktrees` is the one place every caller lists through, and the retry loop exists (`reuse-first`). It retries any failure, as `fetch` does, rather than matching git's message, which a locale can change. The waits are short (100 ms, then 500 ms), since the window lies between two writes of one git process, not a network round trip, so a listing that fails for good, such as outside a repository, costs 0.6 s, not `fetch`'s 3 s.
- **Retry in `git()` for every command whose stderr names a half-written worktree entry.** Not taken: it would cover `git switch`, `git branch -d` and `git worktree add` too, but it matches git's English message, and a command that dies part-way after changing something isn't always safe to run again. No failure outside a listing and a fetch has been seen (`yagni`).
- **Move callers that need only the main worktree to `common_dir`, as `clm-verify` did.** Not taken: once `worktrees` holds against the race those callers are sound as written, and moving them would add a requirement to each of seven components for no behaviour a user sees (`yagni`, `path-ownership`).
- **Read `.git/worktrees/` directly and skip a half-written entry.** Not taken: it copies git's own layout and rules into CLM.

## Outcome

- **`vcs/worktrees-tries-again`**: `worktrees(root)` runs `git worktree list --porcelain -z` again after a failure, at most twice more, waiting 100 ms and then 500 ms, returning the first good listing or the last error, through the loop `fetch` and `fetch_retrying` run, given its own waits. `vcs`'s Contract says so.

## Constraints and edge cases

- The tests fake `git` on `PATH`, as `fetch-tries-again`'s do, so the race is reproduced exactly and no test depends on timing another process.
- An entry left half written for good (a `git worktree add` killed between its writes) still fails the listing after the retries: git itself fails every command on it then, and the error names the entry.
- Other git commands that walk every worktree (`git switch`, `git branch -d`, `git worktree add`) can still meet the race; nothing here changes them.
