---
title: "Three crates format a violation's line by hand"
kind: deferred
components: [check, context, init]
---

`clm-model` now offers the line a violation prints as, `<path>: <rule>: <message>`, as `Violation`'s `Display` (`model/violation-line`), and the root's Design says every crate prints a violation as `clm-model` gives its line. Three crates still write their own copy of the format, so a change to the line in `clm-model` would not reach them:

- `crates/clm-check/src/lib.rs`, `clm_check::format` (`check`), which `affected`, `checkpoint`, `health`, `regenerate`, `sync` and several `check` tests call;
- `crates/clm-context/src/command.rs`, the violation `writeln!` (`context`), and the expected lines in `crates/clm-context/tests/command.rs`;
- `crates/clm-init/src/command.rs`, the violation `writeln!` (`init`).

`gate`'s copy, the expected lines in `crates/clm-gate/tests/exemption.rs`, is mended by `gate`'s build of `20261121-wave-3-land-by-the-rule`.

Each component's build points its copy at `Violation`'s `Display` (`{v}` or `v.to_string()`); `check`'s may then drop `clm_check::format` once its callers print `{v}`. The `apply`, `gate` and `ledger` deltas of `20261115-commands-in-their-crates` already state this for their own commands.
