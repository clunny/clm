---
title: "A land check that builds nothing still waits for one that does"
kind: idea
components: [land, config, ledger]
---

A land whose code is already checked still runs `[land] check` when the tests that read the files it changes are selected (**Readers of checked code** in `land`'s Design). Those runs take the same exclusive locks as a full check (`check-runs-one-at-a-time`, `land-target-lock`), so a spec or docs land that needs a few reader tests waits behind another branch's full build and test run. The code they test hasn't changed since it was checked: only the files they read have.

## Direction: separate building from running

A project can say how to run its tests without building them, as a second, optional command beside `[land] check`. When a land's code key is already checked and its readers are selected, CLM runs that command instead, against the merged tree, without the exclusive lock (or under a shared lock that a building check excludes). The full check keeps the exclusive lock. With no such command set, nothing changes.

This takes the slow part of spec lands out of the queue in any project whose tests read its own docs, which CLM's own repository does heavily (`doc-readers-marked`).

## Toolchains

The setting names a command and assumes nothing about how it works. What it has to do differs by toolchain style:
- **Compiled, test binaries kept** (Rust with `cargo nextest archive` and `--archive-file`, Go with `go test -c`, C and C++ with a built test runner): the full check keeps its test binaries, and the build-free command runs them against the new tree. CLM has to know where those binaries live and that they belong to the checked code key, so the check would record them beside its `checked` record.
- **Compiled on a VM** (Java, Kotlin, C#): compiled test classes or assemblies play the same part; the build tool's own build-free test task can often run them.
- **Interpreted** (Python, JavaScript, Ruby): there is nothing to build, so the build-free command is just the test runner with the selection, and the only gain is skipping the exclusive lock.

A project should never have to set this up to get correct lands: it is only a speed-up, and a test that has to be rebuilt to be valid must stay in the full check.

Related: `code-lands-wait-in-arbitrary-order` (the priority lane covers lands that still need the lock), `only-cargo-projects-get-a-shared-land-build-and-its-lock`.
