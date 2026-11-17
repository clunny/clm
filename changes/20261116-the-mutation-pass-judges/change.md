---
status: applied
resolves: [the-mutation-pass-judges-a-requirement-by-mutants-its-tests-never-reach]
---

## Intent

The mutation pass judges a requirement only by mutants of code its tests run. `.github/scripts/mutate.sh` first runs a Rust block's tests alone under `cargo llvm-cov nextest` and tries, of the diff's mutants and of the further ones, only those with a line of their span among the lines run; StrykerJS's `NoCoverage` mutants count as not tried. A requirement whose tests run lines of its files but none a mutant changes, as when its rule lives in a data or text file such as `clm.toml`, is `not-judged` instead of `unproven`; one whose tests run no line of its files at all, as an empty test, stays `unproven`, so the case the pass was built for is still caught. This is plan work: the mutation pass is phase 3's evidence, and a false `unproven` item costs a build each time it is filed and teaches sessions to delete the pass's findings unread.

## Problem

`verify/mutation-pass` lists every file a requirement's component owns as its `code`, and `ci/mutate-script` tries the diff's mutants and, when none is caught, the first 20 mutants of those files in `cargo mutants --list` order. For a requirement whose tests reach only part of the component, those are mutants of code no citing test runs, so the requirement is filed unproven though its tests are sound. On `backlog/unproven-config-eb2b740e` (`config/unreferenced-detector`) the first 20 mutants of `crates/clm-config/src/` were `Display` impls and getters; the rule is data in `clm.toml`, and the one function its tests run, `detectors()`, had its mutant caught by `unreferenced_detector_is_named_here`. `clm/doc-readers-marked` looks like the same case. The build deleted the item as sound, and the next pass that finds those tests changed would file it again.

No requirement states which mutants count, so this is a spec change (`code-fix-needs-a-written-rule`). No change or code fix since 2026-11-09 fixes it: `20261116-the-mutants-copy-is-a-git-repository` and `20261116-the-mutants-copy-shares` mend the copy the tool tests, not the choice of mutants.

## Approaches

- **Taken: a reach run per Rust block, in the script.** Before a block's first mutant, run the block's tests alone, under the filterset the script already builds, through `cargo llvm-cov nextest` with an lcov report, and keep only mutants whose span holds a line run. It answers exactly the question (which code do the *citing* tests run), needs no tool `coverage.sh` doesn't already install, and stays in the script, since CLM names no tool and verify's plan doesn't change. StrykerJS already analyses coverage per test and reports a mutant no selected test runs as `NoCoverage`; the script counted those as survived, and now counts them as not tried.
- **Rejected: the previous pass's tracefile, `.clm/verify-build/lcov.info`.** It says which lines any test of the suite runs, not which the requirement's tests run, so `Display` impls covered by some other test would still be chosen; and it may be stale or missing.
- **Rejected: running `coverage.sh` before the blocks.** A whole instrumented run of the suite, with the same flaw as the tracefile.
- **Rejected: a separate rule for requirements whose rule lives in data.** Nothing tells a data rule from a code rule mechanically; a requirement whose tests run no mutable line is exactly that case, and the reach rule already makes it `not-judged`.
- **Rejected: accepting it.** The item would be filed again on every pass that finds those tests changed, and each filing costs a build.

## Outcome

- `ci/mutate-script` is modified: mutants of either kind are tried only when the block's tests run a line of their span, learnt from one run of those tests under `cargo llvm-cov nextest`; StrykerJS's `NoCoverage` mutants are neither caught nor survived; a block whose tests run its files but no line a mutant changes is `not-judged`, and one whose tests run none of its files counts its untried mutants as survived; a reach run that fails or writes no report fails the script, so nothing is recorded. Six scenarios are added and one reworded.
- `verify`'s Design says the command tries only mutants of lines the requirement's tests run. Its plan and `verify/mutation-pass` are unchanged.
- `mutation-after-land` records the choice, and the guide's `settings.md` tells an adopter's `[verify] mutate` command the same (`method-in-the-plugin`).

## Constraints

- `land-checks-stay-fast`: the reach run belongs to the mutation pass, after the land; nothing in the land check runs it.
- `ci/tests-time-bounded`: the reach run goes through nextest, whose bound on a test holds.
- The script's tests fake `cargo llvm-cov` as they fake `cargo mutants`, so they install and run nothing real.

## Edge cases and workarounds

- An empty test, or any whose tests run no line of the block's files, tries no mutant and counts the diff's and the first `limit` further mutants as survived, so it is still filed `unproven` (`mutation-after-land`'s founding case). For StrykerJS the same holds when every mutant it reports is `NoCoverage`.
- A mutant's span covering several lines counts as run when any one of them ran: a whole-function replacement is reached when the function is.
- The further mutants are chosen after the reach filter, so the limit of 20 counts mutants the tests run, not mutants listed.
- StrykerJS's further mutants are still the first `limit` lines of each file, which it can't count beforehand; with `NoCoverage` not counted, a block whose tests run none of those lines is `not-judged`, not `unproven`.

## Open questions

- Cost: each Rust block gets one instrumented run of its own tests, on top of its mutants. The instrumented build is shared across blocks; whether the pass's time grows noticeably is for `backlog/a-mutation-pass-that-stops-starts-over-and-has-no-limit-on-its-time` to measure.
- `proven-by-tests`' `#### Untested` block lists what the pass doesn't judge; it doesn't name a requirement whose tests run its files but no line a mutant changes, which this change makes `not-judged`. Adding it edits `ARCHITECTURE.md`, which the human reviews, so it is left here (architecture audit).
