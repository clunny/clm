---
status: applied
resolves: [the-check-runner-s-warning-names-clm-land-when-clm-verify-runs-it]
---

## Intent

Under `clm verify`, `land`'s shared check runner warns that it can't write the check's log as `clm land: warning: ...`, naming a command nobody ran. The runner takes the name of the command that runs it, so `clm verify` says `clm verify: warning: ...` and `clm land` says what it says today; the unnamed form goes once `verify` has moved off it. Resolves `backlog/the-check-runner-s-warning-names-clm-land-when-clm-verify-runs-it`.

## Problem

`clm_land::run_check` writes, live on stderr, when it can't create the check's log, `clm land: warning: can't write the check's log at <path>: <error>; its output goes to stderr alone` (`crates/clm-land/src/check_log.rs`, the `warn` closure of `run`). `clm verify` runs `[verify] check` through the same call (`crates/clm-verify/src/run.rs`, every run of the check and every bisect probe), so under `clm verify` the warning names the wrong command. `land/shared-check-runner` states the behaviour as it is ("whichever command runs it, `clm verify` included"), which wave 3 recorded so the fix would be a spec change of its own.

## Approaches

- **The caller names itself (chosen).** The runner takes the name of the command running it and writes `<name>: warning: ...`. One parameter; no other output of the runner names a command (its waiting line and the check's own output are unprefixed), so nothing else moves.
- **Drop the command from the warning.** Breaks `land`'s Contract, which writes every warning after `clm land: warning: `, and `clm verify`'s output would then hold one warning unlike its others.
- **Return the warning for the caller to print.** The warning must be live, written before the check's output, which can run for minutes; a returned line would come after it.
- **A field on `CheckEnv`.** Callers build `CheckEnv` by struct literal (`CheckEnv { set, unset }` in `verify`), so a new field breaks `verify`'s crate as surely as a new parameter, and it would put a naming concern in a type that says what the check's environment is.

## Outcome

The rename runs expand, then contract (`expand-contract`, `rename-adds-then-removes`), as three steps in this one change:

1. **`land` first** (`land/shared-check-runner`, modified, Needs nothing): `clm-land` offers `run_check_as(name: &str, command: &str, dir: &Path, selection: Option<&Path>, env: &CheckEnv, log: Option<&Path>) -> Result<CheckRun, Error>`, writing `<name>: warning: can't write the check's log at <path>: <error>; its output goes to stderr alone`. `clm land` and `clm land --check-only` pass `clm land`, so they print as today. `run_check(command, dir, selection, env, log)` stays, as `run_check_as("clm land", ...)`, marked `clm:remove-by 20261122-the-check-runner-s`, so `verify`'s crate keeps building. `land`'s Contract lists both forms while both exist (`expand-contract`). `clm land` reaches the runner through the internal `run_in`, so the name is threaded through `run_in` and `check_log::run`'s warning. `land`'s own tests of the runner (`crates/clm-land/tests/runner.rs`, `src/runner.rs`'s unit tests) move to `run_check_as` under their names and citations; "the log's warning" is proved with a name other than `clm land`, and "a land names itself" by `tests/command.rs::check_log_cant_be_created`, which already asserts the line, gaining a `shared-check-runner` citation beside its `check-log` one. `clm land --check-only` gives `clm land` too, though it writes no log, so its warning never shows.
2. **`verify` next** (`verify/verify-run`, modified, Needs `land/shared-check-runner`, which it already lists): every run of `[verify] check`, under `--due`, `--commit` and each bisect probe, goes through `run_check_as("clm verify", ...)` (`crates/clm-verify/src/run.rs`, the one `run_check` call, in `probe`), and `verify-run` states that `clm-verify`'s code and tests call only the named form, so the build moves the two test calls that stand in for a land's check (`tests/run.rs`, `tests/mutate.rs`) too, giving them `clm land`; without that sentence nothing would make `tests/mutate.rs` the build's, and step 3 would break `clm-verify`'s tests. The new scenario "the log's warning names clm verify" proves the name, run in a child process with stderr captured, since the runner writes to the process's stderr.
3. **`land` again** (`land/runner-names-its-caller`, added, Needs `verify/verify-run`): `run_check` and its marker are deleted, proved by a `compile_fail` doc test calling it, as `land.approval/old-calls-gone` proves its removal.

Why one change holds all three: the marker names this change, and `clm check` fails a `clm:remove-by` marker once its change is archived, which happens when every requirement it added or modified is current. Step 3 stays unbuilt until `verify` has moved, so it keeps the change live, and its Need on `verify/verify-run`, pending until `verify`'s build, is what orders `land`'s second build after `verify`'s. Split over two changes, the marker would fail while `verify` still calls `run_check`, or the removal would build before `verify` moves and break `verify`'s crate (`path-ownership`).

No capability test observes the warning (only `crates/clm-land` and `crates/clm-verify` hold its text), so no capability test moves. The change adds no term the glossary covers.

Edge cases and workarounds known now: none beyond the expand-contract's own marker.

## What the requirements no longer state

- `land/shared-check-runner`: "writing, when the log can't be created, the warning `clm land: warning: can't write the check's log at <path>: <error>; its output goes to stderr alone` live on stderr, whichever command runs it, `clm verify` included". The warning now names the command the caller gives; under `clm land` it reads as before, under `clm verify` it reads `clm verify: warning: ...`.

## Constraints

- No build edits another component's code (`path-ownership`): `land` never touches `crates/clm-verify`, and `verify` never touches `crates/clm-land`.
- Built in the order the Needs give: `land` (step 1), `verify` (step 2), `land` (step 3).
- This change belongs after `land`'s and `verify`'s wave-3 builds (`20261121-wave-3-land-by-the-rule`, `20261121-wave-3-verify-by-the-rule`) have landed: both are still pending as this is drafted, and both rewrite the code and the requirements these steps touch. Approved while they are pending, those builds take its steps in with theirs; whether to wait is the human's choice at approval.

## Open questions

None. The builder check and the architecture audit found the Contract dropping `run_check` while the code still offers it (now listed until `runner-names-its-caller`), and `verify`'s test calls tied to no requirement (now in `verify-run`'s text); both are fixed here.
