---
status: applied
resolves: [reading-the-loop-file-creates-its-directory]
---

## Intent

`stored_plan` reads the loop file without creating anything: on a clone that has never run the loop, `clm view queue` leaves no empty `<git common dir>/clm/` behind. Only `clm-loop`'s writes of the loop file make its directory; places, machine samples and the remote mirror keep making it as their own requirements say.

## Problem

`clm_loop::read` and `stored_plan` find the loop file through `state.rs`'s `dir`, which runs `create_dir_all` on `<git common dir>/clm/`. So `clm view queue` on a clone that has never run the loop leaves an empty `clm/` directory in the git directory, though `view.repo/view-queue` says it writes no file. `orchestrator.loop/plan-snapshot` says only that `stored_plan` reads "without fetching", so no requirement of the component holding the fix states that a read creates nothing; `view-queue`'s "no file" doesn't plainly cover a directory, and its test lives in another component's files. By `code-fix-needs-a-written-rule` the rule goes into `plan-snapshot` first.

## Approaches

- **State it in `plan-snapshot` (chosen).** The read is `orchestrator.loop`'s Contract (`stored_plan`), so the rule belongs in its requirement, with a scenario its build cites; the build then makes the read paths locate the file without making its directory, leaving `create_dir_all` to `update`.
- **Code-only fix citing `view.repo/view-queue`.** Its "writes no file" is read as a directory only by stretching it, and the test would sit in `crates/clm-loop/` citing another component's requirement.
- **Accept the empty directory.** It is harmless, but it contradicts what `view-queue` promises (a view writes nothing) and the fix is a few lines, so nothing argues for keeping it.

## Outcome

`orchestrator.loop/plan-snapshot` says `stored_plan` creates and writes no file or directory and that, of `clm-loop`'s code, only what writes the loop file creates `<git common dir>/clm/` (other components' files there, such as `build-places`' places, keep creating it), with a scenario for a clone with no such directory. The requirement becomes pending, and `orchestrator.loop`'s build fixes `state.rs` (the read paths stop calling `create_dir_all`). Nothing else changes: `view.repo/view-queue` already says the view writes nothing, and its tests see no difference beyond the directory's absence (`crates/clm/tests/view_queue.rs`'s `no_loop_yet` counts the directory's entries, which passes either way; its comment that a read may make the directory goes stale, for `view.repo`'s next build to tidy).

Edge cases and workarounds: none.
