---
status: applied
resolves: [run-within-can-report-a-timeout-for-a-command-that-exited-in]
---

## Intent

`run_within` returns the outcome of a command that exited within its limit, even when a process the command started still holds its stdout or stderr open; it reports a timeout only for a command still running when the limit passes. The item's other half, `machine`'s reading of an impossible date such as `2026-02-31` as a sample's time, was code-only against `machine/machine-history` ("its RFC 3339 UTC time") and was fixed by component on `impl/machine` (`bed708e2`) before this change; this change resolves the item for both.

## Problem

`vcs/bounded-runs` says `run_within` behaves as `run` when the command exits within `limit`, and that it returns within one second after `limit` even when a process the command started keeps a pipe open. For a command that exits in time and leaves such a process behind (an `ssh` control master with `ControlPersist` holds the stderr of the `git` command that started it), the two sentences can't both hold: `run` would wait until that process closes the pipe, which may be never. The code settles it by returning `Error::TimedOut` once the limit passes (`crates/clm-git/src/lib.rs`, the readers' `recv_timeout`), so a command that did its work is reported as one that ran out of time; `clm-guard`'s push check reads that as a question it couldn't answer.

## Approaches

- **Taken: return the command's own outcome with what the streams held by the limit.** The command has exited, so its exit status is known and every byte it wrote is in the pipe; only a stray process keeps the stream from ending. The call waits for the streams to close until the limit passes, as `run` would wait, then returns `Ok` or `Failed` as `run` would, from the chunks the readers sent by then and in a further 100 ms. The readers send what they read a chunk at a time, so the bytes are there without the stream's end.
- **Not taken: accept the timed-out error and reword the requirement to say so.** It reports a command that succeeded as one that didn't, which every caller then treats as a failure of the command.
- **Not taken: stop waiting for the streams as soon as the command exits.** It returns sooner, but drops anything the command's own children write after it exits and before the limit, which `run` would include; waiting until the limit keeps the call as close to `run` as the limit allows, and no caller has needed the shorter wait (`yagni`).

## Outcome

- **`vcs`**: `bounded-runs` states what `run_within` returns for a command that exited in time while a process it started holds a pipe, with two scenarios, one for a success and one for a failure. The Design says how the readers make the bytes available without the stream's end.

Edge cases the build knows it needs: the 100 ms after the limit for bytes still in a pipe's buffer, within the requirement's one second.
