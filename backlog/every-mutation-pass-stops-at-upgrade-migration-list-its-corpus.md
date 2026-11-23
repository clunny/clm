---
title: "Every mutation pass stops at upgrade/migration-list: its corpus test can't find clm under cargo llvm-cov"
kind: bug
components: [upgrade]
---

What happened: each `clm verify --due` the build loop ran in this session (four, on four different commits of `main`) passed its check, and then its mutation pass ended with `mutate.sh: cargo llvm-cov failed to find the lines the tests of requirement upgrade/migration-list run`, `clm verify: mutation: .github/scripts/mutate.sh failed with status 1`. The failing step is the coverage run `ci/mutate-script` makes for that requirement's citing tests (`cargo llvm-cov nextest ... --package clm-upgrade`, target dir `.clm/verify-tree/target/mutate-job-1/llvm-cov-target`): the test `clm-upgrade::corpus::a_fresh_project_has_every_migration_done` panics with `<that target dir>/debug/clm is not built`. Its helper builds the binary with a plain `cargo build -p clm`, which writes to the default target directory, and then looks for `clm` beside its own test executable, which under llvm-cov sits in `llvm-cov-target/`. So the requirement's lines can never be found, the script stops, and every due requirement after it in the plan is left: the passes recorded `2 left`, then `90`, `124` and `130 left`, judging only the few requirements reached first.

Expected: the pass reaches a verdict for every due requirement, or at least the failure of one requirement's coverage run doesn't stop the judging of the others; a test that needs the `clm` binary builds it where it then looks for it (the target directory its own executable is in), under any runner.

Rules involved: `verify/mutation-pass` (a pass keeps what it reached and leaves the rest), `ci/mutate-script` (the per-requirement coverage run), and `one-runner-for-the-rust-tests`. The fix most likely lies in `crates/clm-upgrade/tests/corpus.rs`'s binary lookup (component `upgrade`); whether `mutate.sh` should also go on past one requirement whose coverage run fails is a question for `ci`.
