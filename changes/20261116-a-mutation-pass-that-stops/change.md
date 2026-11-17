---
status: applied
from: backlog/a-mutation-pass-that-stops-starts-over-and-has-no-limit-on-its-time
priority: P2
---

## Intent

A mutation pass keeps what it has judged when it stops, uses the machine it runs on, and takes a bounded time. When a pass ends early, because it reached its time limit, a tool failed part-way or someone stopped `clm verify`, the verdicts it reached are recorded, and the `mutated` record names the due requirements it never reached in a new `left` list, which the next pass judges first. `mutate.sh` writes each requirement's line as soon as it is judged, and runs several requirements at once: up to a new `[verify] mutate_jobs` ceiling, cut down to about one per 2 GB of free memory. A new `[verify] mutate_minutes` bounds a pass. Both default to today's behaviour (one at a time, no limit); this repository sets `mutate_jobs = 6` and `mutate_minutes = 60`.

## Problem

Measured on 2026-11-16 (`backlog/a-mutation-pass-that-stops-starts-over-and-has-no-limit-on-its-time`): a pass held 219 requirements after phase 4 moved about 700 tests, took about 46 seconds a requirement, and had judged 177 after 2 hours 16 minutes. Three things made that costly:

- **All or nothing.** `verify/mutation-pass` reads the result file only when the command exits 0, and measures what is due from the commit of the newest `mutated` record. The passes before it failed three times on a test that needed a git repository and once on a hang, each recording nothing, so every retry began again from the first requirement. Stopping that pass by hand would have thrown away two hours the same way.
- **No limit.** A pass runs as long as it has requirements; the human waits on "main is verified" all that time.
- **Serial.** `mutate.sh` judges blocks one after another and `cargo mutants` tries one mutant at a time; the machine (24 cores, 15 GB) showed a load average of 0.5 during the pass.

## Approaches

Keeping what a stopped pass judged (the choice the rest rests on):

- **A record of what is left (taken).** A pass that ends early appends a `mutated` record of what it judged plus `left`, the due requirements it never reached; the next pass's due set is what changed since that record's commit, as today, plus its `left`. One starting commit and one diff stay, and `clm ledger compact`, which keeps the newest record of each commit, keeps working unchanged, since that record holds `left`.
- **Each requirement due since the record that last judged it.** No new field, and no difference between a whole pass and a part of one. Rejected: `compact` would have to keep, for every requirement, the newest record that judged it, or compacting would make old requirements due again, and finding what is due takes one diff per distinct commit instead of one.

Running requirements at once:

- **A ceiling cut down by free memory (taken).** `[verify] mutate_jobs` is a maximum; at the start of a pass `clm verify` allows one block per 2 GB of available memory, at least 1 and at most the ceiling, so a pass beside running builds doesn't push the machine into swap, and a quiet machine runs the ceiling. Memory, not cores, is the limit: each concurrent build of a crate's tests takes one to two gigabytes.
- **A fixed number.** Rejected: the build loop runs builds beside a verify, and a fixed 6 on a 15 GB machine with three builds running would swap.
- **`cargo mutants --jobs`**, several mutants of one block at once. Not taken now: it multiplies the blocks' memory, and blocks at once already uses the machine; measure first.

Bounding a pass:

- **A time limit in minutes, with what is left carried over (taken).** Unset, a pass is unbounded as today. With the carried-over requirements judged first, a limit never starves a requirement.
- **A limit in requirements.** Rejected: requirements vary from seconds to minutes; the human waits on time.

Not judging a test that only moved: rejected, as the item says; most such tests had never been judged.

## Outcome

`ledger` (`deltas/ledger.md`):
- `ledger/mutated-record` (modified): the record holds an optional `left`, a list of requirement IDs, read as empty when absent; a line whose `left` isn't a list of requirement IDs is skipped with a warning; `clm ledger show` prints `<l> left` after `not judged` when `left` isn't empty.

`config` (`deltas/config.md`):
- `config/verify-setting` (modified): `[verify]` takes optional `mutate_jobs` and `mutate_minutes`, each a positive integer, set only beside `mutate`; `clm config get verify` prints `,mutate_jobs=<n>` and `,mutate_minutes=<n>` when set.
- `config/verify-here` (modified): this repository's `[verify]` adds `mutate_jobs = 6` and `mutate_minutes = 60`.

`verify` (`deltas/verify.md`):
- `verify/mutation-pass` (modified): the due set adds the `left` of the newest `mutated` record on `C`'s chain, those still cited at their current version in `C`; the plan holds the carried-over requirements first, in their `left` order, then the rest in ID order. The command gets `CLM_MUTATE_JOBS`, the number of blocks it may run at once: available memory (`MemAvailable` in `/proc/meminfo`) divided by 2 GB, at least 1, at most `mutate_jobs` (1 when unset or unreadable). With `mutate_minutes` set, when the command has run that long `clm verify` ends it and its children (`SIGTERM`, then `SIGKILL` after 10 seconds); a `SIGINT` or `SIGTERM` to `clm verify` during the pass does the same. However the command ends, `clm verify` reads the result lines written so far, ignoring a last line that doesn't parse, and appends a `mutated` record of those verdicts with `left` naming the due requirements without a line, in plan order. It prints `mutation: <k> killed, <u> unproven, <n> not judged`, followed by `, <l> left` when any are left, and `mutation: stopped after <m> minutes` when the limit ended it, `mutation: stopped` when a signal did, or the command's failure as today (`<command> failed with status <n>`) when it exited non-zero, now saying `<k+u+n> recorded` instead of `nothing recorded`. With nothing written and the command failed, it records a `mutated` record with every due requirement under `left`, so `since` still moves and nothing is lost. The outcome line, stdout and the exit status stay untouched, and a signal that stopped the pass then ends `clm verify` with the status it would have had.

`ci` (`deltas/ci.md`):
- `ci/mutate-script` (modified): it appends each block's line to the result file as soon as the block is judged, in one write, and runs up to `CLM_MUTATE_JOBS` blocks at once (1 when unset), each block in its own build directory and with its own `CLM_SLOT`, `CLM_SLOT` plus the block's job number less one; the fixtures run first, once, before any block, and `coverage.sh` after every block. A tool failure in one block lets the blocks already running finish and writes their lines, starts no new block, and exits non-zero naming the tool and the block.

Guide: `docs/guide/settings.md`'s `[verify]` section gains `mutate_jobs` and `mutate_minutes`, with what each trades off.

## Constraints and known edge cases

- Concurrent blocks share the compiler cache but no build directory: each `cargo llvm-cov` coverage run and each `cargo mutants` call gets its own target directory, so disk use grows with `mutate_jobs`.
- Concurrent blocks' tests must pass beside each other, which `tests-run-concurrently` already asks; one that fails only here is a finding, reported as a defect.
- The slots a pass uses, `max_builds + 1` up to `max_builds + mutate_jobs`, belong to no build place and no check.
- A requirement left by a pass whose tests are removed or re-cited at a new version before the next pass is dropped from `left` if no test cites it at its current version, as any requirement is.
- `/proc/meminfo` exists only on Linux; elsewhere the pass runs one block at a time.
- The 60-minute limit is this repository's; a pass that judges nothing within it (each block longer than the limit) would leave everything every time. `clm verify` prints what it left, so the human sees it.

## Open questions

None yet.
