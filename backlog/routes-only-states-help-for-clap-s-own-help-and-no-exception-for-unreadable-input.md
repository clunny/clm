---
title: "`routes-only` asks `--help` of clap's own `help` and allows the root no error of its own"
kind: question
components: [clm]
choices:
  - "Word the scenario as the commands other than the parser's own `help`, and allow the root's errors reading stdin or the current directory"
  - "Turn off clap's `help` subcommand, and move those errors into the crates"
---

Two places where `clm/routes-only@4567a066`, as written, says more than the built code does, found by the spec-auditor on its build:

- **Scenario "every command routes"** runs each command `clm --help` lists with `--help`. That list holds clap's own `help`, and `clm help --help` exits 2 (`unrecognized subcommand '--help'`): `help` takes command names. The test in `crates/clm/tests/routes.rs` skips `help`.
- **"It SHALL print nothing of its own"**: `crates/clm/src/main.rs` still prints `clm defect: reading the report: …` and `clm defect: current directory: …` and `clm config: current directory: …` when the process can't read its standard input or its current directory, which only the process holds, before any call.

Either the requirement's text admits both, or the code changes to fit it.
