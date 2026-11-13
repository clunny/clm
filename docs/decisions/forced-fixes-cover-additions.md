---
title: "A build may make the compile-only fixes an addition to its Contract forces in other components, as it may for a removal"
date: 2026-11-04
status: superseded
superseded_by: coupled-changes-sequence
---

`path-ownership` let a build edit another component's code only for the compile-only and test-fixture fixes a removal forces. An addition can force the same kind of fix: a field added to a public struct breaks every struct literal of it in another crate until the literal names the field. No `#### Needs` can order that: the other component's fix needs the field to exist, and the provider's build can't land the field while the workspace doesn't build. A draft that met it had to design around it (`20261104-detectors-skip-in-the-root`).

The exception now covers an addition too, on the same terms as a removal: the other component's code or tests only build a value that gains a field, and the fix sets the field to the value under which the provider behaves as before (its empty or default value, usually), so that component's behaviour doesn't change. The build names each fix under `## Fixes in other components` in its pull request body, as for a removal, and the build's code review and spec audit read it there (`path-ownership-by-prompt`).

What stays outside the exception, because the fix would decide what the other component does:
- a rename, which `rename-adds-then-removes` writes as an adding requirement and a removing one, the users' moves in between;
- an arm for an added variant that another component matches exhaustively, since the arm is behaviour (the adding requirement must not widen a type a user matches exhaustively), unless that component's requirement already names what the variant selects there and the code it selects is built (see the refinement below);
- an added field with no value under which the provider behaves as before.

A build that meets one of these leaves the requirement unbuilt and files a `question` item with `blocks`, as for a removal another component still uses (`contract-removal-needs`).

Refines `path-ownership` and `contract-removal-needs`.

**Refined 2026-11-06 (`20261106-spec-change-widens-an`): an arm that selects built code its spec names.** `20261106-azure-devops-host` added `azure` to `config/host-setting`, widening `clm_config::Host`, which `clm-host`'s `adapter` matches exhaustively, and gave every `host` requirement for Azure a `#### Needs` on `config/host-setting`. Neither side could build first: `config` couldn't add the arm, and `host` couldn't build before the variant existed. The human settled it: adapter first, then wire. An arm for an added variant is a forced fix when a requirement of the other component, in the specs as applied, names what the variant selects there (that requirement may itself wait on the widening, as `host/azure-selected` does) and the code it selects is built (the requirements that build it current): the arm then selects that code and decides nothing, as in `Host::Azure => Box::new(Azure)` once `host/azure-pull-requests` is current. A change that widens a type another component matches exhaustively orders its builds so:
1. that component's requirements that build the code the new value selects need nothing of the widening and are tested by calling that code directly;
2. the widening requirement needs them, and its build adds the variant and the arm, naming the arm under `## Fixes in other components`;
3. that component's requirements that reach the code through the new value (a setting, a command) need the widening and prove the arm.

Any other arm for an added variant stays outside the exception. The draft prompt and the `propose` skill tell the builder check to look for exhaustive uses of a type a change widens and to order them this way, and the build prompt names the case.

Rejected:
- **A placeholder arm, `Host::Azure => Box::new(NoHost)`, as a forced fix.** It ships `azure` doing nothing, a behaviour no requirement states.
- **One change per side, the variant hidden until the adapter exists.** Two reviews for an order the `#### Needs` blocks already express.

Rejected:
- **Have the propose builder check name the pattern, so each change states the fix.** Every such change would carry the same sentence, and a change written before the check named it, or one whose builder check missed a struct literal, would still leave its build stuck. The rule belongs with the build, which sees the compile error.
- **Make providers' public structs non-exhaustive or give them builders.** A design rule for every library crate to save a one-line fix, and Rust's `#[non_exhaustive]` blocks struct literals outside the crate entirely, tests included.
- **Two changes, the field added first behind a default the users don't see.** Two reviews and a deferred step for a fix that changes no behaviour.
