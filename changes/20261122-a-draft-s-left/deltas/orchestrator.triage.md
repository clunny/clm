---
component: orchestrator.triage
---

## MODIFIED Requirements

### Requirement: draft-finish <!-- base: ecd1b5900349157f -->
`clm draft --finish <change>`, given a repository root that exists, SHALL exit 1 for a `<change>` that is empty or holds `/`, saying it isn't a change ID, before it fetches or changes anything. Otherwise it SHALL fetch `origin`, release the draft's claims as below, free the place `<change>`, printing `freed <change>` when there was one, and, when `origin/main`'s tree holds `changes/<change>/` or `changes/archive/<change>/`, or the local branch `spec/<change>` exists and its tip is an ancestor of `origin/main` that isn't on `origin/main`'s first-parent chain (as for a fold's branch `spec/<batch>` once it has landed), tidy the draft's worktrees and branches. Of the worktrees that have `spec/<change>` checked out, it SHALL remove each under the main worktree's `.claude/worktrees/` that holds nothing outside history, even when it is locked, printing `removed <path>`; SHALL print `kept <path>: uncommitted changes` for one there that holds something else, and `kept <path>: <why>`, git's error, for one whose removal git refuses, going on with the rest; and SHALL keep, printing nothing, one outside `.claude/worktrees/` and the one holding the directory it runs in or the repository root, each counting as kept, and pass over one whose directory is gone. It SHALL then, when no worktree has the local `spec/<change>` checked out, delete it when its tip is on `origin/main` or `origin/spec/<change>`, printing `removed spec/<change>`, and otherwise keep it, printing `kept spec/<change>: commits not on origin`, since deleting it would lose those commits; and delete `spec/<change>` and `autosync/spec/<change>` on `origin`, each leased on the tip the fetch read, printing `removed origin/spec/<change>` and `removed origin/autosync/spec/<change>`, and skipping one that doesn't exist. A worktree SHALL count as holding nothing outside history when it has no unstaged change, no untracked file that isn't ignored, and an index whose tree is the tree of a commit reachable from its `HEAD`, even when that commit isn't `HEAD`. When only the second test holds, it SHALL delete `spec/<change>` on `origin` only when that branch's tip is an ancestor of `origin/main`, and `autosync/spec/<change>` only when it kept no worktree. When neither holds it SHALL remove no worktree and push or delete no branch. The second test SHALL NOT change how `clm draft` judges a change ID held. When the place `<change>` exists and parses, it SHALL delete on `origin` each `propose/<slug>` the place names, leased on the tip it read, printing `released propose/<slug>` before `freed <change>`, except a claim whose tip's committer time is at or after the place's `taken` time plus the expiry, and, while `origin/spec/<change>` exists and neither landed test holds, the claim of an item the change's `resolves` doesn't name whose `backlog/<slug>.md` that branch changes since its merge base with `origin/main`; a claim already gone, or one whose delete the lease refuses, SHALL be skipped without failing. When the place `<change>` exists and parses, it SHALL also print, after its `released` lines and before `freed <change>`, `left <slug>: <change> doesn't resolve it` for each `propose/<slug>` the place names whose `backlog/<slug>.md` `origin/main`'s tree holds and which the `resolves` of the change's `change.md` doesn't name, whether or not it released that claim: that `change.md` read from `origin/main`'s `changes/<change>/` or `changes/archive/<change>/` when it holds one, else from `origin/spec/<change>`'s `changes/<change>/`, and naming no item when none of them holds one that parses. These are the items the draft left, which the build loop records from this output (`orchestrator.loop/return-rules`). When there is no place, or its file doesn't parse, it SHALL release no claim and print no `left` line. It SHALL exit 0, also when there was no place, and 2 when git fails.

#### Scenario: landed draft tidied
- GIVEN a place `20261027-x`, `changes/20261027-x/` in `origin/main`, and a clean, locked worktree at `.claude/worktrees/agent-2` with `spec/20261027-x` checked out
- WHEN `clm draft --finish 20261027-x` runs
- THEN it exits 0, the place is gone, and so is the worktree

#### Scenario: folded draft's branches deleted
- GIVEN `changes/20261027-f/` in `origin/main`, `origin/spec/20261027-f` and `origin/autosync/spec/20261027-f` whose tips aren't in `origin/main`, and a clean worktree under `.claude/worktrees/` with `spec/20261027-f` checked out
- WHEN `clm draft --finish 20261027-f` runs
- THEN it exits 0, the worktree is gone, the local `spec/20261027-f` is gone, and `origin` has neither `spec/20261027-f` nor `autosync/spec/20261027-f`

#### Scenario: landed fold tidied
- GIVEN no place and no `changes/20261027-a-fold/` in `origin/main`, a local `spec/20261027-a-fold` whose tip is the second parent of `origin/main`'s last merge commit, no `origin/spec/20261027-a-fold`, an `origin/autosync/spec/20261027-a-fold`, and a clean, locked worktree under `.claude/worktrees/` with `spec/20261027-a-fold` checked out
- WHEN `clm draft --finish 20261027-a-fold` runs
- THEN it exits 0, the worktree is gone, the local `spec/20261027-a-fold` is gone, `origin` has no `autosync/spec/20261027-a-fold`, and it printed `removed` with the worktree's path

#### Scenario: fold not landed
- GIVEN no `changes/20261027-b-fold/` or `changes/20261027-c-fold/` in `origin/main`, and two clean worktrees under `.claude/worktrees/`, one with `spec/20261027-b-fold` checked out at a commit `origin/main` lacks, the other with `spec/20261027-c-fold` checked out at `origin/main`'s tip
- WHEN `clm draft --finish 20261027-b-fold` and `clm draft --finish 20261027-c-fold` run
- THEN each exits 0, and both worktrees and both local branches are unchanged

#### Scenario: landed fold with uncommitted work
- GIVEN no `changes/20261027-d-fold/` in `origin/main`, a local `spec/20261027-d-fold` whose tip is the second parent of `origin/main`'s last merge commit, checked out in a worktree under `.claude/worktrees/` with an uncommitted change, `origin/autosync/spec/20261027-d-fold`, and `origin/spec/20261027-d-fold` with a commit `origin/main` lacks
- WHEN `clm draft --finish 20261027-d-fold` runs
- THEN it exits 0, prints `kept` with the worktree's path, and the worktree, the local branch, `origin/spec/20261027-d-fold` and `origin/autosync/spec/20261027-d-fold` are unchanged

#### Scenario: worktree left behind by its moved branch
- GIVEN `changes/20261027-m/` in `origin/main`, and a locked worktree under `.claude/worktrees/` with `spec/20261027-m` checked out whose index and files are the tree of the branch's previous commit `P`, the branch having since moved on to a child of `P` made in another worktree, with no unstaged change and no untracked file
- WHEN `clm draft --finish 20261027-m` runs
- THEN it exits 0, prints `removed` with the worktree's path, and the worktree and the local `spec/20261027-m` are gone

#### Scenario: staged work not in history kept
- GIVEN `changes/20261027-n/` in `origin/main`, and a worktree under `.claude/worktrees/` with `spec/20261027-n` checked out and a staged change whose index tree no commit reachable from its `HEAD` has
- WHEN `clm draft --finish 20261027-n` runs
- THEN it exits 0, prints `kept` with the worktree's path, and the worktree is unchanged

#### Scenario: draft waiting for review
- GIVEN a place `20261027-y`, `origin/spec/20261027-y` with its change, no `changes/20261027-y/` in `origin/main`, and a clean worktree with `spec/20261027-y` checked out
- WHEN `clm draft --finish 20261027-y` runs
- THEN it exits 0, the place is gone, and the worktree and `origin/spec/20261027-y` are unchanged

#### Scenario: landed draft's claims released
- GIVEN a place `20261027-r` taken 1 hour ago naming `propose/a` and `propose/b`, both claims committed 1 hour ago, and `changes/20261027-r/` in `origin/main` resolving `a` only
- WHEN `clm draft --finish 20261027-r` runs
- THEN it exits 0, prints `released propose/a` and `released propose/b`, and `origin` has neither `propose/a` nor `propose/b`

#### Scenario: moved item's claim kept while the change waits
- GIVEN a place `20261027-w` taken 1 hour ago naming `propose/a` and `propose/b`, both claims committed 1 hour ago, no `changes/20261027-w/` in `origin/main`, and `origin/spec/20261027-w` whose change resolves `a` and which changes `backlog/b.md`
- WHEN `clm draft --finish 20261027-w` runs
- THEN it exits 0, `origin` has `propose/b` unchanged and no `propose/a`

#### Scenario: another session's later claim kept
- GIVEN a place `20261027-s` taken 6 hours ago naming `propose/a`, with `--expiry 4`, and `origin/propose/a` committed 1 hour ago
- WHEN `clm draft --finish 20261027-s --expiry 4` runs
- THEN it exits 0, the place is gone, and `origin/propose/a` is unchanged

#### Scenario: no place, no release
- GIVEN no place `20261027-q`, `changes/20261027-q/` in `origin/main`, and `origin/propose/a` committed 1 hour ago
- WHEN `clm draft --finish 20261027-q` runs
- THEN it exits 0 and `origin/propose/a` is unchanged

#### Scenario: a local branch with unpushed commits kept
- GIVEN `changes/20261027-u/` in `origin/main`, no `origin/spec/20261027-u`, and a local `spec/20261027-u` that no worktree has checked out, with a commit neither `origin/main` nor any branch on `origin` has
- WHEN `clm draft --finish 20261027-u` runs
- THEN it exits 0, prints `kept spec/20261027-u: commits not on origin`, and the local `spec/20261027-u` is unchanged

#### Scenario: a finish that refuses or keeps
- GIVEN `changes/20261027-k/` in `origin/main`
- WHEN `clm draft --finish` runs: with the ID empty, or `a/b`; in a worktree under `.claude/worktrees/` with `spec/20261027-k` checked out, for `20261027-k`; for `20261027-k` with a second such worktree whose removal git refuses
- THEN with the empty or `a/b` ID it exits 1, saying it isn't a change ID, and `origin` and the clone are unchanged; in the worktree it runs in, it exits 0, prints no line naming that worktree, and that worktree and the local `spec/20261027-k` are still there; with the removal refused, it exits 0, prints `kept <path>: ` with that worktree's path, and removes the other clean worktree under `.claude/worktrees/`

#### Scenario: items the draft left
- GIVEN a place `20261027-l` taken 1 hour ago naming `propose/a`, `propose/b` and `propose/c`, `origin/main`'s `backlog/` holding `a.md` and `b.md` but not `c.md`, no `changes/20261027-l/` in `origin/main`, and `origin/spec/20261027-l` whose change resolves `a`
- WHEN `clm draft --finish 20261027-l` runs, and then runs again
- THEN the first run exits 0 and prints `left b: 20261027-l doesn't resolve it` before `freed 20261027-l`, and no line starting `left a` or `left c`; the second exits 0 and prints no `left` line

#### Scenario: a draft that left every item
- GIVEN a place `20261027-v` naming `propose/a`, `origin/main`'s `backlog/` holding `a.md`, and neither `changes/20261027-v/` nor `changes/archive/20261027-v/` in `origin/main` nor `origin/spec/20261027-v`
- WHEN `clm draft --finish 20261027-v` runs
- THEN it exits 0 and prints `left a: 20261027-v doesn't resolve it` before `freed 20261027-v`

#### Serves
- capability/triage-unattended
- capability/work-concurrently
