---
status: applied
resolves: [narrow-public-items-orchestrator]
---

# orchestrator stops exposing claim_node

## Intent

Take `claim_node` out of `orchestrator`'s Contract, now that `orchestrator.run` claims through `claim_local`, so the crate makes it private and `clm.toml` drops its `--allow` entry for it (`backlog/narrow-public-items-orchestrator`).

## The human's answers

- backlog/narrow-public-items-orchestrator: "Draft now (Recommended)" — take claim_node out of the orchestrator Contract and make it private

## Problem

`clm_orchestrator::claim_node` is public and listed among `orchestrator`'s library calls, but since `20261105-run-cannot-resume-its` nothing outside the crate calls it: `clm claim` goes through `claim`, and `clm run` through `claim_local`. `clm unreferenced` reports it, and `clm.toml` holds a *pending* `--allow` entry for it until the crate is narrowed (`unreferenced-allow-then-narrow`). The build of `orchestrator.run` couldn't narrow it, since the Contract names it.

## Approaches

- **Drop it from the Contract, and state the narrowing in `claim-local` (chosen).** The Contract stops listing `claim_node`; the Design's and `claim-local`'s mentions of it go too, and `claim-local` says the crate exposes no other call that claims a component, so `claim_node` is not public. Modifying `claim-local` puts `orchestrator` behind, so its build makes the function private, drops its re-export in `lib.rs`, deletes the `--allow` entry and its comment in `clm.toml`, and re-cites `claim-local`'s tests. A Contract edit alone builds nothing (`dependency-work-is-a-requirement`).
- **Contract edit only, then a code-only fix on `impl/orchestrator`.** The narrowing would still need a requirement as written to restore (`code-fix-needs-a-written-rule`), and a Contract with no pending requirement leaves nothing to build. Rejected.
- **Keep `claim_node` public as a convenience.** It would keep an `--allow` entry the decision says only shrinks, for a call nobody makes (`yagni`, `no-dead-code`). Rejected.

## Outcome

- `orchestrator` Contract: the library calls list `claim` (a claim with its refusal reason) without `claim_node`.
- `orchestrator` Design: *A claim recorded locally first* says `claim` writes no local branch.
- `orchestrator/claim-local` (modified): `claim` creates no local branch, and the crate exposes no other call that claims a component, so `claim_node` is not public. Its behaviour and scenarios are otherwise unchanged; no capability test observes the change.

## Constraints

- `unreferenced-allow-then-narrow`: the build that narrows `claim_node` deletes its `--allow` entry and comment in `clm.toml` in the same diff, the exception that decision makes to a build editing only its own component's files.
- `path-ownership`: no other component's code uses `claim_node` (`orchestrator.run` uses `claim_local`, `clm claim` uses `claim`), so no other component needs to move (`contract-removal-needs`).

## Edge cases and workarounds

None.
