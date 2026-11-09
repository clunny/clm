---
status: done
resolves: [verify-loses-its-verdict-when-output-fails]
---

## Intent

Once `[verify] check` has exited, `clm verify` records its verdict whatever becomes of its own stdout and stderr. A write to stderr that fails no longer ends a run or the shared check runner, and nothing goes to stdout before the `verified` record is appended, so a stream that fails afterwards (`clm/closed-stdout`, or a full disk) ends the command with the result already in the ledger (`backlog/verify-loses-its-verdict-when-output-fails`, issue #442).

## Problem

A `clm verify --due` run whose slow check had finished exited 101, a Rust panic, with no `verify ` line and no `verified` record, so the next `clm verify --commit` for the same commit ran the whole suite again. `eprintln!` panics when stderr fails, and several of verify's writes to stderr come before the record: the verify tree's waiting line (`crates/clm-verify/src/tree.rs`), a failed selection's `clm affected failed` note (`run.rs`), and in `land`'s shared check runner the check lock's waiting line (`runner.rs`) and the log warning (`check_log.rs`). Bisect probes run after the first check has exited and before the record, so each of their lines can lose the first check's verdict. The runner's copy of the check's output already ignores a failed write (`let _ = stderr.write_all`), so the verdict is lost by CLM's own lines, not the check's output.

No requirement states the behaviour: `verify-run` says only that a run ending before the check exits appends nothing, and `shared-check-runner` says nothing of a failing stderr, so the fix is a written rule first (`code-fix-needs-a-written-rule`).

## Approaches

- **Writes to stderr before the record ignore a failure; stdout waits for the record (chosen).** `verify-run` gains the rule, and `land/shared-check-runner` gains the same for its waiting line, output copy and log warning, which `verify-run` already needs. The record is appended before the outcome line and the notes are printed, as the code already orders it; the rule pins that order. What is printed after the record may still end the command, as `clm/closed-stdout` says: the verdict is safe by then. Small, local, and the same treatment the runner already gives the check's output (`reuse-first`).
- **Stop copying the check's output to stderr, and keep it only in the log.** The issue's reporter suggested it, since a run's output can be thousands of lines. It doesn't fix the fault: that copy already ignores a failed write, and the panics come from CLM's own lines, which would remain. It would also take away a person's view of a long run's progress and make `verify` differ from `land`'s runner. Rejected; listed under Open questions as a separate choice.
- **A CLM-wide rule that no command panics on a failed stderr write.** Wider than the fault: other commands lose nothing they can't redo by running again, and `closed-stdout` already handles the common stdout case in one place. Rejected (`yagni`).
- **Catch the panic in `clm verify`'s dispatch and append the record there.** The verdict lives inside the run when it panics, mid-bisect; recovering it from an unwind means threading partial state through `catch_unwind`. Rejected.

## Outcome

- `verify-run`: a failed write to stderr neither ends the run nor changes its result or exit status before the record is appended; nothing is printed to stdout before the record. New scenario with stderr on `/dev/full` and a contended check lock over two lands, so the run waits, fails and bisects with every stderr write failing.
- `land/shared-check-runner`: a failed write to stderr (its waiting line, output copy and log warning) neither ends the run nor changes what it returns. New scenario with stderr on `/dev/full`, a contended lock and a log that can't be created.
- `verify`'s Design says how (**Output**, in **The run**).

## Constraints

- `clm/closed-stdout` stays as it is: a stdout whose reader has gone still ends the command with 141, and a full disk on stdout still panics with its reason, both only after the record.
- A failed write before the check exits ends nothing either: ending then would leave the check running, unwatched, in the verify tree.
- `land`'s own callers of the runner (`clm land`, `--check-only`) share the runner's rule, so the runner's own writes no longer panic for them either; `clm land`'s other lines (the land lock's waiting line among them) are untouched, since a land that ends early loses nothing it can't redo by running again.

## Edge cases and workarounds

- The scenarios open stderr on `/dev/full`, so they hold on Linux only, as `clm/closed-stdout`'s full-disk scenario does.
- The runner's scenario needs a subprocess (a helper binary or a re-exec of the test) with fd 2 on `/dev/full`, since libtest captures an in-process test's `eprintln!`.
- After the record, `clm verify`'s notes still go out with `eprintln!` (`crates/clm/src/main.rs`), so on a failing stderr a red run still exits 101 once its verdict is recorded; the verify scenario asserts no exit status. Keeping exit 1 there would touch the root component's dispatch for no gain the rule needs.
- The verify tree's own waiting line and the `clm affected failed` note are covered by the rule but not by a scenario; the build mutation-checks them.

## Open questions

- Should `clm verify` stop copying `[verify] check`'s output to stderr and keep it only in the log, as issue #442's reporter suggested? This change keeps the copy, since it doesn't cause the loss and shows a run's progress.
