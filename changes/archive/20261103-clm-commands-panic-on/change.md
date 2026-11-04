---
status: done
resolves: [clm-commands-panic-on-a-closed-stdout-pipe]
---

## Intent

Piping a `clm` command into a reader that exits early, as `clm context <component> | head` does, ends it with a Rust panic and a backtrace note, which sessions read as the command failing. No rule states what should happen, so this change adds one to the root, `closed-stdout`: when the reader of stdout has gone, the command ends with a non-zero status and no panic text.

## Problem

The Rust runtime ignores `SIGPIPE`, so the print returns a broken pipe error, and `println!` panics on it. There are about a hundred such prints, in `crates/clm/src/main.rs` and `clm-runner`. The reader has what it asked for, so the panic is noise: it lands in the session's output and reads like a failure.

## Approaches

- **Panic hook and `catch_unwind` in `main` (chosen).** A dozen lines in one file cover every print, present and future, and destructors still run. Its cost is matching std's panic wording, which the pinned toolchain and a test on the real binary make safe.
- **Restore `SIGPIPE`'s default at start-up.** What most tools get for free, but it needs `unsafe` and `libc`, doesn't exist on the Windows release, skips destructors, and also kills `clm` silently when a child that `clm-git`'s `run` feeds exits before reading its input.
- **A checked write in place of every print.** No string match, but about a hundred edits, and the next `println!` brings the panic back with nothing to catch it.
- **Accept the panic.** The cost is real and recurring, and the fix is small.

## Outcome

`clm context a | head` prints what `head` keeps and nothing else, and exits 141, as a tool killed by `SIGPIPE` reports. Other panics, and a stdout that fails for another reason (a full disk), keep their text.

## Constraints

- Exit 141 stays out of every command's own exit codes.
- `crates/clm/src/main.rs` stays dispatch: the handling lives in its own file beside it, and `main` only calls it.

## Edge cases

- Matching std's message text: the pinned toolchain and the rule's test hold it.
- `clm view` already writes without a panic and exits 1, which the rule allows, so it stays as it is. A report of violations through the same writer also exits 1, ignoring the failed write. `clm status` uses `print!`, so the new handling covers it.
- A failed print to stderr stays a panic: `2>&1 | head` shares one pipe, so its text has nowhere to be seen.
