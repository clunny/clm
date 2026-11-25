---
title: mutate.sh's StrykerJS fixture passes whether or not vitest keeps only the tests that import the mutated file
kind: bug
components: [ci]
---

`mutate.sh` now sets StrykerJS's `vitest.related` to false, so a block's vitest tests run whether or not they import a mutated file. Left true, vitest kept only the tests that import one: five `web` blocks whose tests read files ran no test, StrykerJS exited "No tests were executed", and every pass from `6d6325c6` to `88470edf` failed before most blocks started.

Only a fake-`npx` test (`ci_check.rs::a_typescript_requirement_runs_stryker_on_its_diff_and_its_vitest_tests`) checks the setting, by its text. The real-tool StrykerJS fixture's noticing test imports the function, so it passes with `related` true too, and a pinned StrykerJS that renamed or ignored the option would bring the failure back unseen. `ci/mutate-script` also states the rule only through "with the block's vitest `test` lines alone". A fix states it in the requirement (the block's tests run whether or not they import its files) and gives the fixture a test that imports nothing and only reads a file, which must run.
