---
title: "A command belongs to the crate whose requirements state it: its arguments, what it prints and its exit status"
date: 2026-11-15
status: accepted
---

On `main` at `4f96740b`, 872 of the 1,049 tests in the top-level files of `crates/clm/tests/` proved a requirement of another component, and 725 of those ran the `clm` binary. They sat in the root's files because Cargo gives a built binary only to its own package's tests. No crate offered a call that took a command's arguments and streams: each returned lines or a result, and `crates/clm/src/main.rs` chose the stream, the `clm <command>:` prefix and the exit status, in about 1,850 of its 2,824 lines. So the code that met a component's requirement was partly the root's, edited by every build as wiring, and its proof was wholly the root's. `path-ownership` carried a clause for it, "a file that cites the component" (`own-files-at-the-gate`), and the mutation pass couldn't run such a test in the package it mutated (`mutation-after-land`).

So each crate owns its commands. It defines the command's arguments and offers the command as a call that takes them as the command line gives them, the repository root, the environment as a lookup and a stream each for standard output and standard error, and returns the exit status. `crates/clm` parses with those definitions, applies what holds for every command, loads the settings a crate is passed and routes. A test of a component's requirement calls the component's own crate, in the component's own files.

**The arguments are defined in the crate, not in `crates/clm`.** A component that adds an option must be able to prove it in its own files once the clause is gone; with the definitions in the root, a test of the option could only be the root's. It also lets each moved test keep its arguments and its assertions: only the helper that ran the binary changes.

**Tests move, none is dropped.** `changes/20261115-commands-in-their-crates/moved-tests.txt` is the inventory, made by a script from the markers: 882 lines, one per test and component it cites. A build moves each test under its own name. A test that observes something only a process has stays, with a `question` item.

**What stays the root's.** The commands whose behaviour the root's own requirements state (`clm contribution`, `clm upgrade`, `clm verify`, `clm machine`, `clm agent-harness`, `clm build`, `clm fingerprint`), what holds for every command, and the proof that a routed command prints and exits as its call does. The root measured 174,665 code tokens before this; about 60,000 remain after it, still over its budget of 40,000, most of it `dev-install` and the scans of source and guide text that the plan's phase 5 takes.

**Tests that need no binary** (`ci`, `plugin`, `plugin.agents`, `plugin.panel`: 144 tests that read workflows, scripts and plugin files) move to directories of their own in the root package, as the capability suite has, since they need only a package to run in (`20261115-tests-in-their-own-files`).

Rejected:
- **Leaving the tests in place and moving their ownership**, with a directory or a `tests` glob per component under `crates/clm/tests/`. No code moves and the clause still goes, but every component's proof still depends on the root's binary and `main.rs` keeps other components' behaviour (`components-own-their-tests` rejected it for this repository).
- **Asserting on what the libraries return today.** The moved tests would stop proving the stream and the exit status, which the requirements state.
- **Argument definitions kept in `crates/clm`**, with a plain request type per crate: a new option couldn't be tested by its owner.
- **A shared crate for the call's stream and environment types.** Standard writers and a lookup function need none.
