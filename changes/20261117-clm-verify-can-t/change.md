---
status: applied
resolves: [clm-verify-can-t-file-an-unproven-mutant-when-its-own-clm-binary]
---

## Intent

`clm land` survives its own `clm` being replaced or deleted after the process started. When `std::env::current_exe` gives `<path> (deleted)`, as Linux reports a file renamed over or removed, the land copies the file now at `<path>` when there is one (after a dev-install rebuild, the newer `main`'s `clm`, so the land is still judged by `main`'s rule, `checks-relax-before-use`), and otherwise the running image at `/proc/self/exe`, which the kernel keeps readable for as long as the process runs. That fixes `clm verify`: it files the unproven items its mutation pass found by calling `clm_land::land` in-process at the end of a run that lasts hours, and on a clone under `dogfood-from-main` the dev install has by then renamed a new `clm` over the old one, so `std::env::current_exe` read `<path> (deleted)` and the filing land was refused with `can't copy clm (... (deleted)) to .../land-bin-XXXX/clm (deleted)`.

## Problem

`land/land-binary-copy` copies `std::env::current_exe` before the land's first subcommand, so a check that rebuilds `clm` can't take the binary away mid-land. But `current_exe` on Linux reads the link `/proc/self/exe`, whose text names the file the process was started from, suffixed ` (deleted)` once that file is renamed over or removed. A land started as its own process copies within a second of starting, so the window is small; `clm verify` reaches the land hours after it started, so the window is the whole run, and the build loop runs verify alongside landings (`build-loop-in-clm`), each of which rebuilds the dev binary (`rebuild-on-origin-main`). `verify/files-unproven` retries every unfiled record at the next `clm verify --due`, so nothing is lost for good, but every long run that sees a land would fail the same way, and the items would wait for a short run.

No requirement states that the land copies a binary whose file is gone, so the fix is a spec change (`code-fix-needs-a-written-rule`). The fix lies in `land`'s crate, though the item names `verify`: verify's own code is right, and the land it calls is what fails.

The same run's `mutate.sh failed with status 1`, with 39 mutants not judged, doesn't share this cause: `.github/scripts/mutate.sh` never runs `clm`, and builds in verify's own tree. Nothing in the report says what failed there, so this change designs nothing for it; a recurrence with its log is a new item.

## Approaches

1. **Copy the file now at the path, and the running image only when there is none (taken).** Cutting ` (deleted)` off the path finds the binary that replaced the running one, which is `main`'s newer rule; opening `/proc/self/exe`, rather than reading its text, opens the running image, which the kernel keeps for the process's lifetime, for the case where no file replaced it. One path change in `crates/clm-land/src/exe.rs`, every caller of `land` covered at once (`clm land`, verify's filing, any other in-process land). Systems without `/proc` keep today's behaviour; the dev install, the only thing that replaces `clm` under a running process, runs on this repository's clones, which are Linux.
2. **Always copy `/proc/self/exe` (rejected after the architecture audit).** Simpler, but a long in-process land would then be judged by the older `main` the process started from although the newer `clm` is installed, against `checks-relax-before-use`'s "the rule that guards `main` is `main`'s".
3. **Verify copies its binary once when it starts, and hands the copy to the land.** Narrower and portable, but it adds a parameter to `land`'s call for one caller, copies a binary on every verify run, and leaves every other long in-process caller exposed (`reuse-first`).
4. **Verify files through a `clm land` subprocess.** It would need a `clm` to run, which is the very file that's gone.
5. **Accept it, since the next `clm verify --due` files the record.** Every run long enough to see a land fails the same way, so on a busy clone the items wait for a short run, which may not come for hours.

## Outcome

On Linux, `clm land` lands even when the file it was started from has been replaced or deleted: with the replacing binary when there is one, and with the running image otherwise; `clm verify`'s filing lands after a dev-install rebuild. Elsewhere, nothing changes.

## Constraints and edge cases

- The copy and the refusal take their name from the path with ` (deleted)` cut off, so neither carries the suffix.
- A path that really ends in ` (deleted)` would be cut wrongly; no `clm` is installed under such a name, so the change accepts it.
- Only when no file replaced the binary is the land judged by the rule of the `main` its process started from. That rule can only be older than the newest one, never another project's, and `verify` lands only backlog additions this way; the alternative is the refusal this change removes.
- The new scenario needs a process whose own file is deleted; the land crate's test runs its own test binary from a scratch copy that the child deletes before it calls the land, as `crates/clm-verify/tests/cap.rs` re-runs its own. `CLM_GATE` and `CLM_CHECK` name fakes, so the gate and check never run from the copy. With `CLM_AFFECTED` unset and the fixture's `[land] check`, the land runs `<copy> affected`, which the test binary rejects, so the land falls back to running every test with the fake check; the test should say so in a comment (builder check).
