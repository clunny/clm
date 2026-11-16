---
status: applied
resolves: [rename-s-design-names-a-removed-gate-requirement]
---

## Intent

`rename`'s Design (**Tests.**) places a test that runs the `clm` binary in the root's `crates/clm/tests/rename.rs`, "as `gate`'s do (`binary-tests-apart`)". `gate/binary-tests-apart` is gone, and `rename/commands-in-crate` (`commands-live-in-their-crates`) now says every test of `rename`'s requirements sits in `rename`'s own files and none runs the `clm` binary. This change rewords the paragraph to say that, so the Design no longer cites a removed requirement or contradicts its own component's rule.

## Problem

The Design names a requirement that no longer exists and a test placement that `rename/commands-in-crate` forbids. A builder reading the Design would put binary tests where the requirement says none may be.

## Approaches

- **Reword the Design paragraph** (chosen): edit only **Tests.** to follow `commands-in-crate`. No requirement changes; the code work (moving `crates/clm/tests/rename.rs` into `crates/clm-rename/tests/`) is already what the pending `rename/commands-in-crate` makes a build do.
- Drop the paragraph: loses the line on judging a tree by every violation `clm_model::load` reports, which still guides the tests.

## Outcome

`rename`'s Design says all its tests sit under `crates/clm-rename/tests/`, call `clm-rename` itself (its command call where a requirement states what `clm rename` prints or how it exits) and never run the `clm` binary. Nothing new to build: `rename/commands-in-crate` is already pending and carries the move.

## Constraints

- No edge cases or workarounds known.
