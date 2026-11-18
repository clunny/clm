---
title: "`routes-only` asks `--help` of clap's own `help` and allows the root no error of its own"
kind: deferred
components: [clm]
---

**Answered on 2026-11-17, by the human: "Reword the requirement to admit both."** The work waits for phase 4's closing branch, `backlog/nothing-holds-the-root-s-test-files-to-the-root-s-requirements`, which rewords `clm/routes-only` once `gate/own-files` is built, since that requirement needs `routes-only` and a reword before then would hold it back for a re-cite. The change that carries the answer out names this item in `from` or `resolves`.

Why the text and not the code: `help` is clap's own command and takes command names, so it has no `--help` to ask for; and the three errors are for a standard input or a current directory the process can't read, which only the process holds. Fitting the code to the text would mean handing a standard-input stream to every crate's call for one command's use.

Two places where `clm/routes-only@4567a066`, as written, says more than the built code does, found by the spec-auditor on its build:

- **Scenario "every command routes"** runs each command `clm --help` lists with `--help`. That list holds clap's own `help`, and `clm help --help` exits 2 (`unrecognized subcommand '--help'`): `help` takes command names. The test in `crates/clm/tests/routes.rs` skips `help`.
- **"It SHALL print nothing of its own"**: `crates/clm/src/main.rs` still prints `clm defect: reading the report: …` and `clm defect: current directory: …` and `clm config: current directory: …` when the process can't read its standard input or its current directory, which only the process holds, before any call.

The reword: the scenario "every command routes" leaves out clap's own `help`, and "it SHALL print nothing of its own" excepts an error for a standard input or a current directory the process can't read.
