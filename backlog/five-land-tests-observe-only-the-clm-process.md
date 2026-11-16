---
title: "Five of land's tests observe only the clm process and can't move into its crate"
kind: deferred
components: [land, clm]
origin: 20261115-commands-in-their-crates
blocks: [clm/routes-only]
---

`land/commands-in-crate` moved 10 of the 15 tests `changes/20261115-commands-in-their-crates/moved-tests.txt` lists for `land` into `crates/clm-land/tests/command.rs`, where they call `clm_land::land_command`. Five stay in `crates/clm/tests/land.rs`, because what they observe exists only in a `clm` process (the root's Design: "a test of something only a process has (… the binary's own file, two commands running at once) stays where it is"):

- `check_deletes_the_binary` and `copy_removed` (`land/land-binary-copy`): the land copies its own executable and runs the real gate and `clm check` from the copy, which a check then deletes. Called in a test, the executable is the test binary, which is no `clm`; `clm-land`'s tests have no `clm` binary to run, since Cargo gives it only to `crates/clm`'s tests.
- `overlapping_lands`, `a_callers_target_takes_the_lock_too` and `held_through_the_gate_on_the_merge` (`land/land-target-lock`): two `clm land` processes run at once, the second waiting on the first's lock, and the second test gives each process its own `CARGO_TARGET_DIR`, which two calls in one process can't have.

The first two follow from `crates/clm-land/src/exe.rs` reading `std::env::current_exe()` itself, which the root's Design says a crate's call reads through its arguments instead, while `land/commands-in-crate` names no such argument; the spec change could also say whether the call takes the `clm` binary to copy.

What is needed: say where these scenarios are proven once `clm/routes-only` holds. The human's answer to the two earlier items of this kind (`docs/decisions/commands-live-in-their-crates.md`, 2026-11-15) makes such a scenario the root's, word for word, in the one spec change made once the build run has drained.

## The human's answer (2026-11-15)

The same as for the first items of this kind: a scenario only a `clm` process can show becomes the root's, word for word, in the one spec change that follows the build run (`commands-live-in-their-crates`). The five tests stay where they are until then, and `land`'s workaround flag for them is accepted and deleted.

Answered, so no longer a question: it waits only for the batched spec change, which is due when the phase 4 build run has drained.
