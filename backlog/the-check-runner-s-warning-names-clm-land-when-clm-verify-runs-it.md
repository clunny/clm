---
title: "The check runner's warning names clm land when clm verify runs it"
kind: bug
components: [land, verify]
origin: 20261121-wave-3-land-by-the-rule
---

`land`'s shared check runner (`land/shared-check-runner`, `clm_land::run_check`) writes, live on stderr, when it can't create the check's log: `clm land: warning: can't write the check's log at <path>: <error>; its output goes to stderr alone` (crates/clm-land/src/check_log.rs, the `warn` closure of `run`). `clm verify` runs `[verify] check` through the same call, so under `clm verify` that warning names the wrong command. `land/shared-check-runner` states it as it is (wave 3); this item is the fix.

The design, one spec change holding all three steps (`expand-contract`):
1. `land/shared-check-runner` gains a form that takes the caller's command name: `run_check_as(name: &str, command: &str, dir: &Path, selection: Option<&Path>, env: &CheckEnv, log: Option<&Path>) -> Result<CheckRun, Error>`, writing `<name>: warning: can't write the check's log at <path>: <error>; its output goes to stderr alone`. `clm land` and `clm land --check-only` pass `clm land`, so they print as today. `run_check(command, dir, selection, env, log)` stays, as `run_check_as("clm land", ...)`, marked `clm:remove-by <this change's ID>`. Built by `land` first.
2. `verify`'s requirement that runs the check (`verify/verify-run`) states that the warning reads `clm verify: warning: can't write the check's log at <path>: <error>; its output goes to stderr alone`, and Needs `land/shared-check-runner`. verify's build moves its three calls to `run_check_as("clm verify", ...)`: crates/clm-verify/src/run.rs (the `run_check` call, at run.rs:72 at 735d292a), tests/run.rs:213 and tests/mutate.rs:971.
3. A new `land` requirement, for example `land/runner-names-its-caller`: `clm-land` offers the runner only in the form that takes its caller's name, so no caller's warning names another command; it Needs `verify/verify-run`. Built by `land` after verify's build: it deletes `run_check` and its marker.

Why one change must hold them: the marker names the change, and `clm check` fails a `clm:remove-by` marker once its change is archived, which happens when every requirement it added or modified is current. Step 3's requirement stays unbuilt until verify has moved, so it keeps the change live, and its Need on `verify/verify-run`, modified in the same change and so pending until verify's build, is what orders land's second build after verify's. Split over two changes, or put into a branch whose change land's first build completes, the marker fails while verify still calls `run_check`, or the removal builds before verify moves and breaks verify's crate. Propose it after branches A and C of wave 3 have landed, since both rewrite these requirements.
