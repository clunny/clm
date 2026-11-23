---
status: applied
resolves: [a-ctrl-c-during-clm-verify-s-filing-after-a-run-that-didn-t-pass]
---

## Intent

`clm verify` catches `SIGINT` and `SIGTERM` from its outcome line, whatever that line is, until `clm-verify`'s call returns, not only from a `verify passed ` line. So the stop that `verify/files-unproven` promises for the filing, and the cleanup after a failed step, hold after `verify not-due` and after a red run too, which are the cases where the filing most often runs. Resolves `backlog/a-ctrl-c-during-clm-verify-s-filing-after-a-run-that-didn-t-pass`.

## Problem

`clm/verify-command` sets the call's stop request on `SIGINT` or `SIGTERM` only once the outcome line starts `verify passed `, because the window was first drawn around the mutation pass, which only a passing run goes on to. `verify/files-unproven` then made the filing run at the end of every `clm verify --due` in `local` mode with `[verify] mutate` set, whether or not a run was due. A session ending runs `clm verify --due --event work-done`, which usually prints `verify not-due`, so this is the common path. On that path, and after a red run, no handler is installed. A Ctrl-C while the filing lands its branch takes the default action and ends `clm verify` itself. The filing never deletes its branch `unproven/<sha>` here or on `origin`, never removes its temporary worktree and never prints `unproven: not filed:`, so the branch is left on `origin`.

## Approaches

- **Catch from the outcome line, whatever it says (chosen).** `clm-verify`'s Contract already says that `announce` is called once with the outcome, before the mutation pass and the filing, and never on an error. So the outcome line marks where everything that should stop in an orderly way begins. `clm` installs the handlers in `announce` for every outcome, which is one condition removed from its code. Where nothing follows the outcome (a `--commit`, `--bisect` or `--import` call, `pipeline` mode, `verify off`), the window only covers the clearing of the verify tree and the return. A signal there sets the request and the exit status is 128 plus its number, which is the status the signal would have given anyway.
- **Catch only when the filing can follow** (`--due` without `--dry-run`). `clm` would have to know the mode and whether `mutate` is set, which `clm-verify` reads from `origin/main`'s settings (`verify/settings-from-main`). Either `clm` would read them a second time, or `clm-verify`'s Contract would have to tell its caller. That is more Contract and code than the case is worth, and buys nothing over the chosen approach (`yagni`).
- **Have `clm-verify` install the handlers.** This moves process-wide signal handling into a library the tests call in-process, where a handler would outlive the call and leak between tests. `clm/verify-command` already owns the signal, and `verify` owns only the stop request (`Call`).

## Outcome

- `clm/verify-command`: the window starts at the outcome line, whatever it is, and covers the mutation pass, the clearing of the verify tree and the filing. It gains a scenario, "stopped while it files after a run that wasn't due".
- No other component changes. `verify/files-unproven` already says what the filing does with a stop request, and `verify`'s Contract already calls `announce` before the filing.
- The clm build also mends the comments in `crates/clm/src/main.rs` (`verify`, its `announce`) and `crates/clm/src/stop_signal.rs`'s module comment that state the `verify passed ` window.

## Edge cases and workarounds

- `SIGTERM` sent to `clm verify` alone while the filing's land runs does not end the land, since `clm land` has no stop request. The land runs to its end, the filing finishes or reports as it would, and `clm verify` then exits 143. This is what `verify/files-unproven` already says for the passing path.
- A signal that arrives after a red outcome line, on a call that files nothing, now makes `clm verify` exit 128 plus its number once the call returns, where it used to be ended by the signal. The status seen by a shell is the same.
- A signal before the outcome line still ends `clm verify` as it ends any process (`a_signal_before_the_pass_ends_it_as_any_process` stays true).

## Constraints

- The outcome line, stdout and the exit status are unchanged when no signal arrives (`verify/files-unproven`, `verify/mutation-pass`).
- Only the root's code changes (`path-ownership`).

## Open questions

None.
