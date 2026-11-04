---
title: "Code no test exercises is read from an lcov file the project produces"
date: 2026-10-28
status: accepted
---

Under `proven-by-tests`, code no test runs serves no proven requirement, which makes line coverage a bloat signal worth showing. CLM builds projects in any language, and running each language's coverage tool would be one integration per tool (`dead-code-detectors` made the same call for dead code).

So `clm health` reads one format: an lcov tracefile (`SF:`, `DA:<line>,<count>`, `end_of_record`), which `cargo-llvm-cov --lcov`, `c8`, `nyc`, `coverage.py`'s `lcov` reporter and most other tools write. The repository's `clm.toml` names the file's path under `[health] lcov`; how and when the project produces it is the project's business, outside `clm check` and `clm land`, which never need it. With no path, or no file at the path, the signal reports that and nothing else, so a repository that produces no coverage loses one signal and nothing more.

Only files some node's `paths` own are reported, so vendored or generated code the tool happens to instrument never shows. Runs of unexercised lines are reported, not percentages: a run is something to read and cut, a percentage is something to argue about.

Rejected:
- **Running the tool from `clm health`.** A command per language, a toolchain the report would then need, and a slow report.
- **Cobertura or JSON formats.** lcov is the format every tool can write; one parser is enough (`reuse-first`).
- **Making it a detector in `clm check`.** Coverage below a bar is a judgment, and a build that lowers it can't always fix it; the signal is advisory (`health-advises`).

This repository doesn't yet produce an lcov file; `backlog/lcov-in-this-repository` records the step.
