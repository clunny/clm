---
component: verify
---

## ADDED Requirements
### Requirement: culprit-recheck
When `verify-bisect` names a culprit land whose `landed` record says it re-checked with `none` or `overlap` (`ledger/landed-recheck`), `clm verify` SHALL print on stderr, after its outcome line, `culprit <short sha> re-checked <how> at land; its branch's check and main's were composed from <head key> and <base key>`, `<how>` being `nothing` for `none` and `the overlap, <n> files` (or `the overlap, every test`) for `overlap`, and the keys those of the composed `checked` record for its code key, abbreviated to 12 characters, or `unknown` for both when the fold holds no such record. It SHALL read the culprit's `landed` record and its `checked` records from the fold's lands and checks. It SHALL say nothing of the failure's cause: a missing fact in the graph, the code and a flaky test all fail this way. Its stdout and the `verified` record SHALL be unchanged.

#### Scenario: a culprit that ran no check
- GIVEN `verify-bisect`'s breaking-land fixture, with `l2`'s `landed` record holding `recheck: none` and a composed `checked` record for its code key with basis `h` and `b`
- WHEN `clm verify --due` runs with a trigger holding
- THEN stdout is as `verify-bisect` says, and stderr holds `culprit <l2> re-checked nothing at land` and both keys

#### Scenario: a culprit that ran its own selection
- GIVEN the same with `recheck: selected`
- WHEN `clm verify --due` runs
- THEN stderr holds no `culprit` line

#### Needs
- ledger/landed-recheck
- ledger/composed-check
