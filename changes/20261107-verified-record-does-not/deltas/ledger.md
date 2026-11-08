---
component: ledger
---

## MODIFIED Requirements

### Requirement: verified-record <!-- base: 844d746fea056391 -->
The fold, `fold` and `fold_records` alike, SHALL parse `verified` records and hold them by `commit`, the newest by `at` first, skipping with a warning a line whose `passed` isn't a boolean, whose `mode` isn't `local`, `pipeline` or `import`, or whose `commit`, `since`, `lands` or `culprit` holds anything but full 40-hex lowercase commit IDs, as git writes them (or null where null is allowed): the fold looks records up by exact string, so it would hold an uppercase ID but never match it. `append` SHALL write it like any record, unsigned in every repository: a `verified` record holds no land by itself (`land/verify-stop` reads the backlog item triage takes it in as), so it is a report, as the kinds other than `approved` and `landed` are. `clm ledger show` SHALL print a `verified` record as `verified <commit> passed|failed <writer> <at>`. `clm ledger compact` SHALL keep the caller's newest `verified` record for each commit and drop older ones for the same commit. `Fold::unresolved_reds(chain)`, given a first-parent chain of commits oldest first, SHALL return the newest record of each commit after the last one whose newest record passed (of every commit when none passed) whose newest record failed, in chain order, for `verify`, `land` and `orchestrator.triage` to read alike.

#### Scenario: by commit
- GIVEN two `verified` records for commit `c`, the second red with culprit `l`, and one for commit `d`
- WHEN the fold reads them
- THEN it holds both for `c`, the red one first, and one for `d`

#### Scenario: bad result
- GIVEN a `verified` line whose `passed` is `"yes"`
- WHEN the fold reads it
- THEN it skips the line with a warning naming it

#### Scenario: uppercase commit
- GIVEN a `verified` line whose `commit` is 40 uppercase hex digits
- WHEN the fold reads it
- THEN it skips the line with a warning naming it

#### Scenario: shown
- GIVEN a passing `verified` record for commit `c`
- WHEN `clm ledger show` runs
- THEN it prints `verified <c> passed <writer> <at>`

#### Scenario: unresolved reds
- GIVEN a chain `a`, `b`, `c`, `d`, a passing record for `a`, a red one for `b`, a red then a passing one for `c`, and a red one for `d`
- WHEN `unresolved_reds` reads the chain
- THEN it returns `d`'s record alone, since `c`'s newest record passed

#### Scenario: compacted
- GIVEN the caller's two `verified` records for commit `c`
- WHEN `clm ledger compact` runs
- THEN only the newer remains

#### Serves
- capability/slow-tests-after-land
