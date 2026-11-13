---
component: web
---

## MODIFIED Requirements
### Requirement: fresh-after-write <!-- base: 0e8d21431df746b5 -->
Every request the app makes to load data again after the Worker answered one of its writes (an approve, a Request changes, a comment, a proposal, a withdrawal, an answer to a backlog item or a capability's decision), on each response after which it loads again, success or refusal, SHALL carry `fresh` set to a delivery of the app's own, a random UUID from `crypto.randomUUID()`, new for each write, so the Worker answers from live state fetched anew rather than its minute-old copy. The copy kept for offline use SHALL be stored under the request's path without `fresh`, as for a notice's refresh.

#### Serves
- capability/follow-from-browser
- capability/approve-intent

#### Needs
- web.worker/fresh-live-state

#### Scenario: approve
- GIVEN `spec/x`'s review with fingerprint `f1` and no approvals, and a Worker that answers the approve with 200
- WHEN the reviewer taps Approve
- THEN the review is requested again from `/branches/spec/x/review?fresh=<u>`, `<u>` a UUID of lowercase hex digits and `-`, and the copy kept for offline use is stored under `/branches/spec/x/review`

#### Scenario: a new delivery per write
- GIVEN the backlog on screen, and a Worker that answers each proposal with 200
- WHEN two proposals are sent one after the other
- THEN each is followed by an implementation request carrying `fresh`, and the two values differ

#### Scenario: a refused write
- GIVEN `spec/x`'s review drawn with fingerprint `f1`, and a Worker that answers that approve with 409
- WHEN the reviewer taps Approve
- THEN the review is requested again with `fresh` set

## ADDED Requirements
### Requirement: answer-from-app
The app SHALL give Home and a backlog item's page the answer buttons' `Answering` (`web.views/answer-buttons`): the implementation state's `answers`, none until it arrives, whether the app is offline, and an `onAnswer` that runs as a write (**Loading and refresh**), so no refresh runs during it. `onAnswer` SHALL post `{"answer": <answer>}` to `POST /api/repos/<o>/<r>/backlog/<slug>/answer`; on 200 it SHALL ask for the implementation state again (`fresh-after-write`) and redraw the view on screen from it, keeping its scroll; on 422 it SHALL say the answer was refused, on 404 that the item is no longer in the backlog, on any other status that the answer wasn't recorded, with the status, and when the request isn't sent that it is offline, each as text under the buttons, asking for nothing again; a 401 shows the sign-in page, as for the app's other writes.

#### Scenario: answered on the item's page
- GIVEN repo `o/a` whose backlog holds question `q` with choices `yes` and `no`, and a Worker that answers the answer with 200 and then the implementation state with the answer `yes` by `alice/web`, `mine` true
- WHEN `#/o/a/backlog/q` is opened and `yes` is tapped
- THEN exactly one request posted `{"answer": "yes"}` to `/api/repos/o/a/backlog/q/answer`, the implementation state was requested again with `fresh`, and `yes` is drawn chosen

#### Scenario: answered on Home
- GIVEN the same repo, with Home on screen
- WHEN `yes` is tapped under `q`
- THEN one request posted `{"answer": "yes"}` to `q`'s route, and Home draws `yes` chosen

#### Scenario: refused
- GIVEN `q`'s page, and a Worker that answers the answer with 422 `{"error": "answer"}`
- WHEN `yes` is tapped
- THEN a message under the buttons says the answer was refused, the buttons stay enabled, and no implementation state was requested after the post

#### Scenario: offline
- GIVEN `q`'s page drawn from the copy kept for offline use, offline
- WHEN the page is shown
- THEN its answer buttons are disabled

#### Serves
- capability/follow-from-browser
- architecture/per-writer-ledgers

#### Needs
- web.views/answer-buttons
- web.views/home-waiting-on-you
- web.views/backlog-item-page
- web.worker/answers
