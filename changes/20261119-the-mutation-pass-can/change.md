---
status: applied
resolves: [the-mutation-pass-can-t-judge-a-rule-whose-code-lives-in-a-test]
---

## Intent

The mutation pass judges a requirement whose rule is code in its test's own file by mutants of that file, instead of filing it unproven or leaving it `not-judged`. When a Rust block's tests run no mutant of its code files, `ci/mutate-script` lists the mutants of the files holding its tests under a crate's `tests/` with `cargo mutants --Zmutate-file <file> --json`, tries those its tests run, each written into a copy of the tree of the block's own job and tested there with `cargo nextest run`, and writes `killed` or `survived <n>` as for the code's mutants. An empty test still runs no mutant of its file, since the tool never mutates a `#[test]` function, so it still reads as unproven. This is plan work under `backlog/a-passing-check-proves-less-than-the-spec-claims`: the mutation pass is phase 3's evidence.

## Problem

`backlog/the-mutation-pass-can-t-judge-a-rule-whose-code-lives-in-a-test`: the pass filed `vcs/one-git-layer` unproven (`backlog/unproven-vcs-75853921`) after mutating `crates/clm-git/src/`, which its tests never run. The requirement's rule is the scan in `crates/clm-git/tests/scan.rs`, and `cargo mutants` never mutates a test target, so no test change could kill those mutants and the item asked for work that can't be done. A pass by hand of the scan's 21 mutants killed every one. The vcs build then added `clm:reads` markers for the crates the workspace test reads, which leaves the next pass `not-judged` (`20261117-the-mutation-pass-files`), so a later weakening of those tests goes unseen. This repository has many such rules: scans of its own sources and specs written as tests.

## Approaches

- **Tell it from an empty test, and write `not-judged`** (by coverage of the test file's lines outside the citing tests). Taken by the item as acceptable, but it judges nothing: it is what the `reads` markers already give the vcs case, and a weakened scan would still go unseen.
- **Mutate the test file** (taken). `cargo mutants` 27.1.0, the pinned version, lists any Rust file's mutants with `--Zmutate-file <file> --json`, with each one's span, replacement and diff, skipping `#[test]` functions; checked on `crates/clm-git/tests/scan.rs`, it lists 31 mutants of `scan`, `matches`, `group`, `rust_files`, `sorted`, `read`, `fixture` and `write`. The script applies each itself and runs the block's tests with nextest, reading the outcome from nextest's exit status (0 passed, 100 a test failed, 101 the build failed). `cargo llvm-cov` reports test files' lines with `--no-default-ignore-filename-regex`, so the same filter by lines run applies.
- **Move such rules out of test files into library code** that `cargo mutants` mutates. Rejected: it reshapes every component whose rule is a scan, for a tool's limit, and puts test-only code in shipped crates.
- **Try test-file mutants beside the code's.** Rejected: a test file's helpers build its fixtures, and a mutant of one is caught however little the test asserts, so tried beside the code's mutants they would turn survivors into `killed`. They are tried only when the code offers no mutant the tests run and the tests run no other component's code (a misplaced test, which the pass should still report), which is when the verdict would otherwise be `survived` untried or `not-judged`.

## Outcome

- `ci/mutate-script` gains the test-file path, its fixture (the Rust fixture crate gains a test file whose helper one test notices and one doesn't, so an unstable `--Zmutate-file` listing or a misread nextest status fails the fixture), its failure cases and nine scenarios. Every existing scenario keeps its name and stays true: the fake `cargo mutants --Zmutate-file` lists no mutant unless a scenario says so.
- `ci`'s Design gains the paragraph *A rule in a test file*; `verify`'s Design says such a requirement is judged by its test file's mutants.
- The verdicts keep their meaning. A requirement now `not-judged` only because of `reads` markers whose rule is code in its test file becomes judged.

## Constraints

- `--Zmutate-file` is an option `cargo mutants` marks unstable; the pinned version and the fixture contain that.
- The mutated copy is per job and inside the script's temporary directory, so blocks at once (`tests-run-concurrently`) and the checkout never see a mutant.
- Only files under a crate's `tests/` named by the block's `test` lines: `#[cfg(test)]` modules in `src/` are left as they are (`yagni`); no rule is known to live there.

## Decisions

`docs/decisions/mutation-after-land.md` gains a dated section: it supersedes, for blocks whose rule is code in their test file, the 2026-11-16 and 2026-11-17 outcomes that left such blocks not judged (raised by the architecture audit).

## Open questions

None.
