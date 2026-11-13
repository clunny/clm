---
component: replica
---

## MODIFIED Requirements

### Requirement: validate-writes <!-- base: a4cd62e97f391a2b -->
`validate` SHALL refuse a decision on an entry its tip's `CAPABILITIES.md` or `ARCHITECTURE.md` doesn't hold, or on a tip whose specs don't load, as `Unknown`; on a tip that holds `docs/allowed_signers`, as `Signed`; and on an entry whose version isn't the one named, as `Stale`, in that order; and otherwise SHALL give the flags `clm_effort::unsettled` names for the entry over its ledgers. It SHALL refuse a proposal whose slug its backlog holds, its `resolved` holds, or an open proposal of its fold has, as `Taken`, and a proposal naming a component its graph doesn't hold, as `Invalid`.

#### Scenario: a reworded capability
- GIVEN a replica whose `CAPABILITIES.md` holds `x` at version `v2`
- WHEN a decision on `x` naming `v1` is validated
- THEN it is refused as `Stale`

#### Scenario: signed before stale
- GIVEN a replica whose tip holds `docs/allowed_signers` and `x` at `v2`
- WHEN a decision on `x` naming `v1` is validated
- THEN it is refused as `Signed`

#### Scenario: a taken slug
- GIVEN a replica whose backlog holds `export-csv`
- WHEN a proposal titled "Export CSV" is validated
- THEN it is refused as `Taken`

#### Serves
- architecture/git-is-truth
- architecture/humans-review-intent

### Requirement: open-proposals <!-- base: 53a076e120a88e8b -->
The replica's implementation view SHALL carry `proposals`: the open `proposed` records of its ledger fold, each with `id`, `writer`, `at`, `slug`, `title`, `body` and `/nodes` and no `mine`, sorted by `at` then `id`, leaving out each whose slug a backlog item of its tip has or a change of its tip, archived or not, names in `resolves`, which are what its snapshot's `backlog` and `resolved` hold.

#### Scenario: filed, resolved and withdrawn
- GIVEN a fold holding `proposed` records `p1` with slug `s1` by `alice/web`, `p2` with slug `s2`, `p3` with slug `s3` and `p4` with slug `s4`, a `withdrawn` record naming `p4`, and a tip with `backlog/s2.md` and an archived change whose `resolves` names `s3`
- WHEN the views are recomputed
- THEN `proposals` lists `p1` only, with its `id`, `writer`, `at`, `slug`, `title`, `body` and `/nodes` and no `mine`, and the snapshot's `resolved` holds `s3`

#### Scenario: order
- GIVEN open proposals `p2` and `p1` at 09:00 and `p3` at 08:00
- WHEN the views are recomputed
- THEN `proposals` lists `p3`, `p1`, then `p2`

#### Scenario: none
- GIVEN a replica holding no ledger
- WHEN the views are recomputed
- THEN `proposals` is empty

#### Serves
- capability/propose-intent
- capability/follow-from-browser
- architecture/git-is-truth

#### Needs
- ledger/proposal-records

### Requirement: open-answers <!-- base: 2de5b2ad67fe0277 -->
The replica's implementation view SHALL carry `answers`: the open answers of its ledger fold (`ledger/answer-records`) whose item its tip's backlog holds, each with `id`, `writer`, `at`, `item` and `answer` and no `mine`, sorted by `at` then `id`.

#### Scenario: answered, then resolved
- GIVEN a fold holding open answers `a1` naming `q` and `a2` naming `gone`, and a tip whose backlog holds `q` and not `gone`
- WHEN the views are recomputed
- THEN `answers` lists `a1` only, with its fields and no `mine`

#### Serves
- capability/follow-from-browser
- architecture/git-is-truth

#### Needs
- ledger/answer-records

### Requirement: validate-answers <!-- base: 78c682ffebf38ab1 -->
`validate` SHALL refuse an answer naming an item its tip's backlog doesn't hold as `Unknown`, and as `Invalid` an answer to a `question` item that isn't one of its `choices` (so any answer to a question without choices), an answer to a `human-task` item other than `done`, and any answer to an item of another kind.

#### Scenario: what an item offers
- GIVEN a tip whose backlog holds question `q` with choices `yes` and `no`, question `r` with none, `human-task` `h` and `idea` `i`
- WHEN answers `yes` to `q`, `maybe` to `q`, `yes` to `r`, `done` to `h`, `yes` to `h`, `done` to `i` and `yes` to `nope` are validated
- THEN the first and fourth pass, `nope` is refused as `Unknown`, and the rest as `Invalid`

#### Serves
- capability/follow-from-browser
- architecture/git-is-truth

#### Needs
- model/backlog-choices
