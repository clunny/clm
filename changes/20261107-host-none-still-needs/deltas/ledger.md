---
component: ledger
---

## MODIFIED Requirements

### Requirement: append <!-- base: 58df8447d5ba0960 -->
`append(root, record)` SHALL add the record, with a fresh `id`, the writer and the time unless it already has them, as a record `sign` returned does (`record-signing`), as one JSON line at the end of `records.jsonl` on the writer's branch `clm/ledger/<person>/<clone>`, creating the orphan branch on first use, without touching the working tree or the index, and SHALL push the branch to `origin` without force, on every host, `none` included, whose `origin` is the clone's own remote (`host/none-keeps-a-remote`). When `origin` can't be reached it SHALL keep the commit and report the record as unpushed. It SHALL never rebase or force-push a ledger branch.

#### Scenario: first record
- GIVEN a clone with a bare `origin` and no ledger branch
- WHEN `append` runs with an `approved` record
- THEN `origin` has `clm/ledger/<writer>` whose `records.jsonl` holds one line with `kind` `approved`, the writer and an `at`, and the working tree is unchanged

#### Scenario: second record
- GIVEN that clone after the first record
- WHEN `append` runs with a `checked` record
- THEN `records.jsonl` on `origin` holds both lines in order, and the branch has two commits

#### Scenario: no host
- GIVEN a clone with `host = "none"` whose `origin` is a bare repository in its git directory, and no ledger branch
- WHEN `append` runs with an `approved` record
- THEN `origin` has the writer's ledger branch holding the record, and it reports the record as pushed

#### Scenario: offline
- GIVEN a clone whose `origin` URL points nowhere
- WHEN `append` runs
- THEN the local branch has the record, it reports the record as unpushed, and it doesn't fail
