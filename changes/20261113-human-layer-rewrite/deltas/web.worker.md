---
component: web.worker
---

## MODIFIED Requirements

### Requirement: proposals <!-- base: e608c36e45ee2a27 -->
`POST /api/repos/{o}/{r}/proposals` with `/title`, `/body` and `/nodes` SHALL, through `writes-to-object`, append a `proposed` record with the title, its kebab-case slug, the body and the components to `clm/ledger/<login>/web` through the Git Data API with the requester's own token, making no other write to the repository, and answer 200 with the record's `id`. It SHALL answer 422 with `{"error": <field>}`, writing nothing, for a title that is empty or over 120 characters once trimmed or holds a line break, a body over 8,000 characters, a component the replica's graph doesn't hold, or a slug that is empty, and 409 with `{"error": "taken"}`, writing nothing, when the slug is taken: the replica's backlog holds an item of that slug, its `resolved` holds it, or its fold holds an open proposal with it. `DELETE /api/repos/{o}/{r}/proposals/{id}` SHALL, through `writes-to-object`, append a `withdrawn` record naming `{id}` when the requester's own web ledger, read at its tip, holds a `proposed` record of that `id` no `withdrawn` record in it names, and answer 404, writing nothing, for any other `id`. The implementation response SHALL carry `proposals`: the implementation view's (`replica/open-proposals`), in its order, each with `mine` added, whether the writer is the requester's `<login>/web`. Each route SHALL make the access check every API route makes.

#### Scenario: proposed
- GIVEN a signed-in user `alice` who can read `o/a`, whose graph holds component `a`
- WHEN she posts `/title` `"Export CSV"`, `/body` `"Let me export."` and `/nodes` `["a"]`
- THEN the writes made are exactly a blob, a tree, a commit and a ref update on `clm/ledger/alice/web`, with her token, the new `records.jsonl` ends with a `proposed` line with slug `export-csv`, that title, body and components, and the answer carries its `id`

#### Scenario: refused
- GIVEN the same, where the replica's tip holds `backlog/export-csv.md`
- WHEN she posts the same, then `/title` `"Dark mode"`, `/body` `""` and `/nodes` `["nope"]`
- THEN the first answers 409 and the second 422, and nothing is written

#### Scenario: taken by a change
- GIVEN the same, where the replica's `resolved` holds `export-csv`
- WHEN she posts `/title` `"Export CSV"`, `/body` `""` and `/nodes` `[]`
- THEN the answer is 409 with `error` `taken`, and nothing is written

#### Scenario: the slug
- GIVEN the same
- WHEN she posts `/title` `"  Dark mode: on/off!  "`, a title of 70 `a`s, a title of 62 `a`s, a space and `bb`, then `/title` `"!!!"`, each with `/body` `""` and `/nodes` `[]`
- THEN the first three append `proposed` lines with slugs `dark-mode-on-off`, 64 `a`s and 62 `a`s, and the last answers 422 with `error` `slug`, asking neither GitHub nor the object

#### Scenario: withdrawn
- GIVEN `clm/ledger/alice/web` holding `proposed` `p1` and `clm/ledger/bob/web` holding `proposed` `p2`
- WHEN `alice` deletes `p2`, then `p1`, then `p1` again
- THEN the first answers 404 writing nothing, the second appends `withdrawn` naming `p1` to her ledger, and the third answers 404 writing nothing

#### Scenario: open proposals
- GIVEN an implementation view whose `proposals` are `p1` by `alice/web` and `p2` by `bob/web`
- WHEN the implementation response is built for `alice`
- THEN `proposals` lists `p1` with `mine` true, then `p2` with `mine` false, each with the view's other fields as given

#### Serves
- capability/propose-intent
- capability/follow-from-browser
- architecture/git-is-truth

#### Needs
- ledger/proposal-records
- replica/open-proposals
- replica/validate-writes
- web.worker.replica/writes-through-object

### Requirement: decide <!-- base: fb1ef5808834faa3 -->
`POST /api/repos/<o>/<r>/capabilities/<slug>/decide` SHALL, through `writes-to-object`, append a `decided` record for `capability/<slug>`, with the body's `value` and `decision` and `settles` the flags `clm_effort::unsettled` names over the default branch's tip and the ledgers the object holds, to `clm/ledger/<login>/web` through the Git Data API with the requester's own token, making no other write, and SHALL answer 200 with the record. It SHALL answer 422 and write nothing for a body without a string `version` of 8 lowercase hex digits, a `value` of `high`, `medium` or `low` and a `decision` of `keep`, `cut`, `redesign` or `accept` (checked before any request to GitHub), for a tip whose specs don't load, for a `<slug>`, percent-decoded, that `CAPABILITIES.md` doesn't hold at the tip, and, with the body `{"error": "signed", "command": "clm capabilities decide <slug> --value <value> --decision <decision>"}`, for a repository whose default branch's tip holds `docs/allowed_signers`; it SHALL answer 409 and write nothing when `version` isn't the capability's version now. The 422 cases are checked before the 409 one. When the object can't read the tip, or the append fails, the route SHALL answer 502 and write nothing more.

#### Serves
- capability/review-capability-cost
- capability/follow-from-browser
- architecture/git-is-truth
- architecture/humans-review-intent

#### Needs
- ledger/decision-records
- replica/validate-writes
- web.worker.replica/writes-through-object

#### Scenario: decide
- GIVEN a capability `x` at version `v` with one unsettled flag `f`
- WHEN the user decides it with `{"version": "v", "value": "low", "decision": "cut"}`
- THEN the writes made are exactly a blob, a tree, a commit and a ref update on `clm/ledger/<login>/web`, with the user's token, the new `records.jsonl` ends with a `decided` line for `capability/x` by `<login>/web` with value `low`, decision `cut` and settles `[f]`, and the answer is that record

#### Scenario: reworded since
- GIVEN the page of `x` was drawn at version `v`, and `x` has since been reworded
- WHEN the user decides it naming `v`
- THEN the Worker answers 409 and makes no write

#### Scenario: bad body
- WHEN the user decides `x` with no body, a `version` that isn't 8 lowercase hex digits, a `value` of `huge` or a `decision` of `maybe`
- THEN the Worker answers 422 each time, makes no request to GitHub or the object and no write

#### Scenario: signed
- GIVEN a default branch whose tip holds `docs/allowed_signers`, and a stale `version`
- WHEN the user decides `x` with `{"version": <stale>, "value": "high", "decision": "keep"}`
- THEN the Worker answers 422, not 409, with `error` `signed` and `command` `clm capabilities decide x --value high --decision keep`, and makes no write

#### Scenario: unknown capability
- WHEN the user decides `nope`, which `CAPABILITIES.md` doesn't hold, and in a second run decides `x` on a tip whose `specs/c/SPEC.md` doesn't parse
- THEN the Worker answers 422 both times and makes no write

#### Scenario: GitHub fails
- GIVEN a GitHub that fails the read of the default branch's ref
- WHEN the user decides `x`
- THEN the Worker answers 502 and no write is made

### Requirement: architecture-decide <!-- base: 296cef09d47a5540 -->
`POST /api/repos/<o>/<r>/architecture/<slug>/decide` SHALL do for `architecture/<slug>` what `decide` does for a capability: append a `decided` record for it, with the body's `value` and `decision` and `settles` the flags `clm_effort::unsettled` names for it, to `clm/ledger/<login>/web` with the requester's own token and no other write, and answer 200 with the record; answer 422, writing nothing, for the bodies `decide` refuses, a tip whose specs don't load, a `<slug>`, percent-decoded, that `ARCHITECTURE.md` doesn't hold at the tip, and, with the body `{"error": "signed", "command": "clm architecture decide <slug> --value <value> --decision <decision>"}`, a repository whose default branch's tip holds `docs/allowed_signers`; answer 409, writing nothing, when `version` isn't the entry's version now, the 422 cases checked first and in `decide`'s order; and answer 502 as `decide` does.

#### Serves
- capability/review-capability-cost
- capability/follow-from-browser
- architecture/git-is-truth
- architecture/humans-review-intent

#### Needs
- ledger/decision-records
- effort/unsettled-flags
- replica/validate-writes
- web.worker.replica/writes-through-object

#### Scenario: decide
- GIVEN an architectural constraint `e` at version `v` with one unsettled flag `f`
- WHEN the user decides it with `{"version": "v", "value": "medium", "decision": "redesign"}`
- THEN the writes made are exactly a blob, a tree, a commit and a ref update on `clm/ledger/<login>/web`, the new `records.jsonl` ends with a `decided` line for `architecture/e` with value `medium`, decision `redesign` and settles `[f]`, and the answer is that record

#### Scenario: not an entry
- WHEN the user decides `nope`, which `ARCHITECTURE.md` doesn't hold, or `e` at a version it no longer has
- THEN the Worker answers 422 for the first and 409 for the second, and makes no write

#### Scenario: signed
- GIVEN a default branch whose tip holds `docs/allowed_signers`
- WHEN the user decides `e` with `{"version": "v", "value": "high", "decision": "keep"}`
- THEN the Worker answers 422 with `error` `signed` and `command` `clm architecture decide e --value high --decision keep`, and makes no write

### Requirement: answers <!-- base: 7d19919b8b677c1c -->
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
- architecture/git-is-truth

#### Needs
- ledger/answer-records
- replica/validate-answers
- replica/open-answers
- web.worker.replica/writes-through-object
