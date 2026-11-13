---
component: web.worker
---

## ADDED Requirements
### Requirement: answers
`POST /api/repos/{o}/{r}/backlog/{slug}/answer` with `/answer` SHALL, through `writes-to-object`, append an `answered` record with `item` `{slug}` and that answer to `clm/ledger/<login>/web` through the Git Data API with the requester's own token, making no other write to the repository, and answer 200 with the record's `id`. It SHALL answer 404 with `{"error": "unknown"}` for an item the replica refuses as `Unknown`, and 422 with `{"error": "answer"}` for a missing or non-string `/answer` or one the replica refuses as `Invalid` (`replica/validate-answers`), writing nothing. The implementation response SHALL carry `answers`: the implementation view's (`replica/open-answers`), in its order, each with `mine` added, whether the writer is the requester's `<login>/web`. The route SHALL make the access check every API route makes.

#### Scenario: answered
- GIVEN a signed-in user `alice` who can read `o/a`, whose backlog holds question `q` with choices `yes` and `no`
- WHEN she posts `/answer` `"yes"` to `/api/repos/o/a/backlog/q/answer`
- THEN the writes made are exactly a blob, a tree, a commit and a ref update on `clm/ledger/alice/web`, with her token, the new `records.jsonl` ends with an `answered` line naming `q` with answer `yes`, and the answer carries its `id`

#### Scenario: refused
- GIVEN the same
- WHEN she posts `/answer` `"maybe"` to `q`'s route, then `"yes"` to `/api/repos/o/a/backlog/nope/answer`
- THEN the first answers 422 with `error` `answer` and the second 404 with `error` `unknown`, and nothing is written

#### Scenario: open answers
- GIVEN an implementation view whose `answers` are `a1` by `alice/web` and `b1` by `bob/web`
- WHEN the implementation response is built for `alice`
- THEN `answers` lists `a1` with `mine` true, then `b1` with `mine` false

#### Serves
- capability/follow-from-browser
- architecture/per-writer-ledgers

#### Needs
- ledger/answer-records
- replica/validate-answers
- replica/open-answers
