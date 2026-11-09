---
title: "A land that can't skip its check still runs every test in a component the change touches, and often every test"
kind: idea
components: [check.affected, verify, check.health]
---

## Problem

`clm affected` selects tests at the level of components and files: everything owned by an affected component and by every component that depends on it, plus files whose markers name one. In this repository many changes select every test, through `[affected] all` (`Cargo.toml`, `Cargo.lock`, the toolchain pin, `crates/clm/src/**` and the check script). After `20261108-toolchain-neutral-land-builds`, a land that has to check runs the overlap of two such selections, so a coarse selection still makes it cost a full check. Both the overlap rule and batching (`queued-lands-each-run-a-full-check`) get cheaper as the selection gets finer.

## Settled (2026-11-08, with the human)

- **Order.** Third of three: after the overlap rule and after batching.
- **Deterministic, by the binary.** Selection stays a computation `clm affected` makes from facts, never a model's judgement (`deterministic-enforcement`). Agents maintain the facts (`paths`, `depends_on`, `req:` and `clm:reads` markers, `[affected] all`); CLM's checks keep them honest, as `check/undeclared-imports` and `dangling-markers` do now.
- **Who owns what.** Which components and files a change can affect is CLM's: it reads the spec graph and knows no language. Which individual tests run for those files is the project's: its check script maps `CLM_SELECTION`'s files onto its own test runner. CLM may accept a finer file-to-test mapping from a project in a format CLM defines, and must work without one.
- **A failure doesn't name its cause.** A test that fails after the land, on a land whose check the selection narrowed or skipped, may be a missing fact in the graph, a code bug or a flaky test. CLM reports the evidence (`verify/culprit-recheck`) and leaves the diagnosis to whoever takes the report.
- **This repository's catch-alls are project work.** Narrowing `crates/clm/src/**` and the other `[affected] all` globs, so a change to one command's dispatch doesn't select every test, is this repository's own job, done under the same rules as any project.

## Prior art

| Approach | Examples | Deterministic | Across languages |
|---|---|---|---|
| Declared build graph | Bazel, Buck, Pants, Nx, Turborepo, Gradle | yes | only with that build system |
| Per-test coverage maps | Datadog Test Impact Analysis, Microsoft Test Impact Analysis (.NET), pytest-testmon, Ekstazi (Java) | yes, while the map is fresh | per-language plug-ins; miss files read that aren't code |
| Predictive (machine learning) | Launchable, Develocity Predictive Test Selection, Meta's predictive test selection | no | yes |
| Recording the files each test process opens | Microsoft BuildXL's file-access monitoring; `strace` or eBPF on Linux | yes | yes, at the operating-system level |

CLM's current approach is the first family's, with the graph declared in CLM's own formats, which is why it works in any language.

## Approaches

- **Audit the declared graph by tracing.** During `[verify]`'s full runs, which are slow anyway, record which files each test process opens, and report a test that read a file its declared facts don't connect it to, without presuming whether the fact or the test is wrong. It works for any language, sees files read at run time (docs, specs, config) and could check the hand-written `clm:reads` markers or, later, replace them. Its limits: it's precise only per process (a test binary holding many tests is traced as one, unless each runs in its own process), each operating system traces differently, and it slows the run.
- **A finer mapping from the project.** A project that can map files to individual tests (from its build tool or a coverage map) hands CLM that mapping, and the selection narrows to those tests.
- **Signals in `clm health`.** Flag lands that keep running every test, naming the catch-all or the chain that selected them (`land/check-time-report` already names a step's chain), so a project sees where narrowing would pay.

## Open questions

- Is per-process tracing precise enough for Rust, where `cargo nextest` runs each test in its own process, and for runners that don't?
- Which operating systems must tracing support, and what happens on the others: no audit, or a coarser one?
- Does a finer mapping replace `clm:reads` markers, or only check them?
- Which of this repository's catch-alls can narrow first, and what does each one cost today? `20261108-land-check-timing`'s step times and chains are the evidence.
