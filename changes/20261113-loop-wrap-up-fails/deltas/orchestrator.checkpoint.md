---
component: orchestrator.checkpoint
---

## ADDED Requirements

### Requirement: wrap-up-delete-loses-race
When `origin` refuses a delete `clm wrap-up` makes of a done branch there (`checkpoint`: a landed `spec/*` or `impl/*` branch, an `autosync/spec/<id>` or a `propose/<slug>`) because the branch is no longer at the tip the fetch read, `clm wrap-up` SHALL read that branch on `origin` again and go on. A branch `origin` no longer has SHALL count as done, since another session or command deleted it after the fetch and nothing is left to delete, and a branch whose tip moved SHALL be kept, as the lease means it to be. It SHALL print no `removed` line for either, SHALL still delete every other done branch on `origin` and print that branch's `removed` line, and the refusal SHALL NOT make it exit 2 or end `wrap-up incomplete`. When a delete fails and `origin` still has the branch at the tip the fetch read, the failure is git's and it SHALL exit 2, as for any git failure.

#### Serves
- capability/work-concurrently

#### Scenario: a claim deleted since the fetch
- GIVEN `origin/propose/q` where `backlog/q.md` isn't in `origin/main`, the landed `origin/spec/y` of "landed work cleaned up", nothing else outstanding, and an executable `git` first on `PATH` that, before it runs the first `push` naming `origin`, deletes `propose/q` on `origin`, and runs the real `git` for every call
- WHEN `clm wrap-up` runs
- THEN it prints `removed origin/spec/y`, no `removed` line naming `propose/q` and `wrap-up ok`, exits 0, and `origin` has neither branch

#### Scenario: a claim retaken since the fetch
- GIVEN the same, with a `git` that moves `propose/q` on `origin` to a new commit instead of deleting it
- WHEN `clm wrap-up` runs
- THEN it prints `removed origin/spec/y`, no `removed` line naming `propose/q` and `wrap-up ok`, exits 0, `origin/propose/q` is at the new commit and `origin` has no `spec/y`

#### Scenario: a delete git fails
- GIVEN the same branches, with a `git` that fails every `push` naming `origin` with `fatal: unable to access` and exit 128, changing nothing on `origin`
- WHEN `clm wrap-up` runs
- THEN it exits 2 and doesn't print `wrap-up ok`
