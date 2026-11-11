---
component: orchestrator.run
---

## ADDED Requirements

### Requirement: prompt-unopened-pr
`clm run`'s prompt SHALL tell the session, where it says to open the pull request, that a host may cut a long body (Azure DevOps keeps 4000 characters), so the body puts what a reviewer needs first and stays short; that when `clm pr open` fails, the session still lands, since nothing waits on a pull request (`land-locally`), and does not retry `clm pr open` by another route; and that whenever `clm pr open` or `clm pr body` says the body is kept at a path, the session's final message names that path, since the build's worktree, and `.clm/pr-body.md` with it, is removed once it lands.

#### Scenario: prompt
- GIVEN a ready component `b`
- WHEN `clm run b` runs
- THEN the prompt it writes contains `4000 characters`, says that when `clm pr open` fails the session still lands with `clm land`, and says that the final message names the path `clm pr open` says the body is kept at

#### Serves
- capability/build-from-spec

#### Needs
- host/keep-unopened-body
