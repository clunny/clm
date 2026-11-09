---
title: clm verify loses a finished run's verdict when writing its output fails
kind: bug
components: [verify]
issues: [442]
---

When a `clm verify --due` run's check has finished but `clm verify` then fails to write to
its output streams (stdout or stderr redirected to a file that stops accepting writes, or a
closed stream), the process panics with exit 101 before it appends the `verified` record.
The check's result is lost: no outcome line, nothing in the ledger, and the next
`clm verify --commit` for the same commit runs the slow suite again from scratch.

Expected: once the check has exited, its result reaches the ledger whatever happens to the
process's output. A failed write to stderr (the waiting line in `tree.rs`, the
`clm affected failed` note in `run.rs`, the log warning in `clm-land`'s `check_log.rs`)
does not end the run, and the record is appended before anything that can fail on output.
`clm/closed-stdout` may still end the command afterwards. No requirement says so today:
`verify-run` only says a run that ends before the check exits appends nothing, so this
needs a spec branch (`code-fix-needs-a-written-rule`).
