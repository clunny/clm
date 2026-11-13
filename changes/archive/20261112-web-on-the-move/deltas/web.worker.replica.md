---
component: web.worker.replica
base: { contract: "87483aa7bc26a7ee" }
---

## MODIFIED Contract
Crate `clm-web-replica`: the Durable Object class `Replica`, one per repo, named `<o>/<r>` (`id_from_name`), which runs `replica`'s core on Cloudflare with a GitHub source and the object's own storage (`web-repo-replica`). `clm-web` re-exports the class and reaches each object through a `Replicas` platform trait, which tests fake. Its routes, all internal (the Worker is the only caller, through the `REPLICA` binding):

- `POST /event`: a verified webhook's event name, delivery and body, plus the installation's id. Answers 202 once the event is applied.
- `GET /view/<name>?fresh=<0|1>`: the view `replica`'s `view` gives, as JSON, with `sha` and `computed_at`, or 503 `{"preparing": <share>}`. With `fresh=1` it first brings itself up to the tip and recomputes if anything was stale.
- `POST /write`: a web write that needs the replica to validate (decide, propose, withdraw, answer), the requester's login and token, in the body `web.worker/writes-to-object` gives. It brings itself up to the tip, validates, appends the record through `web.worker.platform`'s `ledger::append` with the requester's token, applies it, and answers with the status and body the Worker's route answers, or 503 `{"preparing": <share>}` while it has no views. A decide, a proposal or an answer recomputes before the answer.

A read also tells the object the repo has open pages; the object keeps that for 5 minutes.

## MODIFIED Requirements
### Requirement: writes-through-object <!-- base: b47769434d7f84a7 -->
A write SHALL be validated against the replica after the object has brought itself up to the tip, by reading the default branch's ref and applying any push it hasn't seen, and SHALL be appended with the requester's token and applied to the replica before the answer. A decide, a proposal or an answer SHALL recompute before the answer, so a read right after it shows the record. An answer SHALL be validated by `replica/validate-answers`, `Unknown` answering 404 `{"error": "unknown"}` and `Invalid` 422 `{"error": "answer"}`, and appended as an `answered` record with `item` and `answer`, its `id` in the 200 answer. While the replica has no views to answer (`Preparing`), a write SHALL answer 503 `{"preparing": <share>}` as a read does, before it reads the ref, start the build when none is under way, and append nothing. When the object can't read the ref, or its tip isn't that ref's commit after applying the push, a write SHALL answer 502 and append nothing, so nothing is judged against a tip it knows to be behind; a tip that moved with files still to fetch has moved.

#### Scenario: an answer read back
- GIVEN a replica whose backlog holds question `q` with choices `yes` and `no`
- WHEN an answer `yes` to `q` is written through the object, and the implementation view is read right after
- THEN the view's `answers` holds it

#### Needs
- replica/validate-answers
- replica/open-answers
- replica/validate-writes
- replica/apply-what-changed

#### Scenario: a decision shows at once
- GIVEN a replica whose capabilities view shows `x` undecided
- WHEN a decision on `x` is written, and the capabilities view is read
- THEN the view shows the decision

#### Scenario: the tip can't be read
- GIVEN a replica holding `x`, and a GitHub that fails the read of the default branch's ref
- WHEN a decision on `x` naming its version, a proposal and a withdrawal are written
- THEN each answers 502, and nothing is appended

#### Scenario: still preparing
- GIVEN an object that holds nothing
- WHEN a proposal is written
- THEN it answers 503 `{"preparing": 0}`, the alarm is set, and nothing is appended

#### Scenario: reworded since the page was drawn
- GIVEN a replica that hasn't applied a push rewording `x`, which GitHub's default branch already holds
- WHEN a decision on `x` naming its old version is written
- THEN the object applies the push, refuses the decision as stale, and appends nothing
