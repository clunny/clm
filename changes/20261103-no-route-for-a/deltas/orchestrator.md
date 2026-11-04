---
component: orchestrator
---

## MODIFIED Requirements

### Requirement: claim <!-- base: e1e68bdb9ac423b5 -->
`clm claim <component>` SHALL push a new commit on `origin/main` to `impl/<component>` atomically, failing if the branch appeared or moved since it was read. It SHALL exit 1 without pushing when the component isn't one `clm next` would print, saying why: the items and requirements that block it, that it is converged, or that it is claimed. When it is converged, it SHALL also say that a small change needing no spec change takes no claim and lands from a branch of its own with `clm land`. It SHALL take over a lapsed claim, and a claim whose tip is an ancestor of `origin/main`. When the push is rejected, it SHALL read the branch's tip on `origin` again. If the tip is the claim commit, it SHALL succeed. If the tip differs from the one it expected (none for a new claim, the released tip for a takeover), it SHALL exit 1 saying someone else claimed the component first. Otherwise it SHALL push the same commit again, at most twice more, waiting 1 second before the first of these and 2 seconds before the second, and judge each rejection the same way, except that a tip still unchanged after the last push SHALL exit 2 with git's reason and a line saying the failure may be transient and to run the same command again, without saying the component was claimed by someone else.

#### Scenario: race
- GIVEN two sessions that both see component `b` as ready
- WHEN both run `clm claim b`
- THEN exactly one exits 0 and the other exits 1

#### Scenario: already claimed
- GIVEN `origin/impl/b` committed 1 hour ago
- WHEN `clm claim b` runs
- THEN it exits 1 and pushes nothing

#### Scenario: takeover
- GIVEN `origin/impl/b` committed 5 hours ago
- WHEN `clm claim b` runs
- THEN it exits 0 and the branch tip is the new claim commit

#### Scenario: merged takeover
- GIVEN `origin/impl/b` committed 1 hour ago and merged into `origin/main` with a merge commit
- WHEN `clm claim b` runs
- THEN it exits 0 and the branch tip is a new claim commit whose parent is `origin/main`

#### Scenario: rejected for another reason
- GIVEN an `origin` whose `pre-receive` hook rejects every push, printing `declined by test`
- WHEN `clm claim b` runs
- THEN the hook ran three times, it exits 2, its message contains `declined by test` and `may be transient` and not `claimed by someone else`, and `origin` has no `impl/b`

#### Scenario: rejected twice
- GIVEN an `origin` whose `pre-receive` hook rejects the first two pushes, printing `fatal error in commit_refs`
- WHEN `clm claim b` runs
- THEN it exits 0 and `origin/impl/b` is the claim commit

#### Scenario: why it isn't ready
- GIVEN component `c` whose only pending requirement `c/x` is in the `blocks` of an existing `backlog/q.md`, and component `d` with every requirement current
- WHEN `clm claim c` and `clm claim d` run
- THEN each exits 1, the first naming `backlog/q`, and the second saying `d` is converged and naming `clm land` for a change that needs no spec change

#### Scenario: pending dependency
- GIVEN component `a` with an uncited requirement, depending on component `b` with an uncited requirement
- WHEN `clm claim a` runs
- THEN it exits 0

#### Scenario: blocked by a need
- GIVEN component `a` whose only pending requirement `a/x` needs uncited `b/y`
- WHEN `clm claim a` runs
- THEN it exits 1 without pushing, saying `a` is blocked by `b/y`

#### Needs
- view/blockers-view
- view/status-report
