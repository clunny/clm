---
component: replica
---

## ADDED Requirements
### Requirement: snapshot-backlog-text
Each item of the snapshot's `backlog` SHALL carry `body`, the item file's text after its frontmatter with leading and trailing blank lines removed; `priority`, `P1` to `P5`, `P5` when the item sets none; and `choices`, the item's choices (`model/backlog-choices`), empty when it has none.

#### Scenario: an item in full
- GIVEN a tip with `backlog/q.md` holding `kind: question`, `priority: P2`, `choices: [yes, no]` and the body `Should we?`, and `backlog/i.md`, an `idea` with no priority
- WHEN the views are recomputed
- THEN the snapshot's `q` carries `body` `Should we?`, `priority` `P2` and `choices` `yes` and `no`, and `i` carries `priority` `P5` and no choices

#### Serves
- capability/follow-from-browser

#### Needs
- model/backlog-choices

### Requirement: open-answers
The replica's implementation view SHALL carry `answers`: the open answers of its ledger fold (`ledger/answer-records`) whose item its tip's backlog holds, each with `id`, `writer`, `at`, `item` and `answer` and no `mine`, sorted by `at` then `id`.

#### Scenario: answered, then resolved
- GIVEN a fold holding open answers `a1` naming `q` and `a2` naming `gone`, and a tip whose backlog holds `q` and not `gone`
- WHEN the views are recomputed
- THEN `answers` lists `a1` only, with its fields and no `mine`

#### Serves
- capability/follow-from-browser
- architecture/per-writer-ledgers

#### Needs
- ledger/answer-records

### Requirement: validate-answers
`validate` SHALL refuse an answer naming an item its tip's backlog doesn't hold as `Unknown`, and as `Invalid` an answer to a `question` item that isn't one of its `choices` (so any answer to a question without choices), an answer to a `human-task` item other than `done`, and any answer to an item of another kind.

#### Scenario: what an item offers
- GIVEN a tip whose backlog holds question `q` with choices `yes` and `no`, question `r` with none, `human-task` `h` and `idea` `i`
- WHEN answers `yes` to `q`, `maybe` to `q`, `yes` to `r`, `done` to `h`, `yes` to `h`, `done` to `i` and `yes` to `nope` are validated
- THEN the first and fourth pass, `nope` is refused as `Unknown`, and the rest as `Invalid`

#### Serves
- capability/follow-from-browser
- architecture/per-writer-ledgers

#### Needs
- model/backlog-choices
