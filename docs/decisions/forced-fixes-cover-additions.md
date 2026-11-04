---
title: "A build may make the compile-only fixes an addition to its Contract forces in other components, as it may for a removal"
date: 2026-11-04
status: accepted
---

`path-ownership` let a build edit another component's code only for the compile-only and test-fixture fixes a removal forces. An addition can force the same kind of fix: a field added to a public struct breaks every struct literal of it in another crate until the literal names the field. No `#### Needs` can order that: the other component's fix needs the field to exist, and the provider's build can't land the field while the workspace doesn't build. A draft that met it had to design around it (`20261104-detectors-skip-in-the-root`).

The exception now covers an addition too, on the same terms as a removal: the other component's code or tests only build a value that gains a field, and the fix sets the field to the value under which the provider behaves as before (its empty or default value, usually), so that component's behaviour doesn't change. The build names each fix under `## Fixes in other components` in its pull request body, as for a removal, and the build's code review and spec audit read it there (`path-ownership-by-prompt`).

What stays outside the exception, because the fix would decide what the other component does:
- a rename, which `rename-adds-then-removes` writes as an adding requirement and a removing one, the users' moves in between;
- an arm for an added variant that another component matches exhaustively, since the arm is behaviour (the adding requirement must not widen a type a user matches exhaustively);
- an added field with no value under which the provider behaves as before.

A build that meets one of these leaves the requirement unbuilt and files a `question` item with `blocks`, as for a removal another component still uses (`contract-removal-needs`).

Refines `path-ownership` and `contract-removal-needs`.

Rejected:
- **Have the propose builder check name the pattern, so each change states the fix.** Every such change would carry the same sentence, and a change written before the check named it, or one whose builder check missed a struct literal, would still leave its build stuck. The rule belongs with the build, which sees the compile error.
- **Make providers' public structs non-exhaustive or give them builders.** A design rule for every library crate to save a one-line fix, and Rust's `#[non_exhaustive]` blocks struct literals outside the crate entirely, tests included.
- **Two changes, the field added first behind a default the users don't see.** Two reviews and a deferred step for a fix that changes no behaviour.
