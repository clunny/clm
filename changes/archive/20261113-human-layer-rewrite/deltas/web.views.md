---
component: web.views
---

## MODIFIED Requirements

### Requirement: queue-states <!-- base: 340afeed7165da3f -->
The implementation view SHALL know exactly the queue states `ready`, `claimed` and `blocked` (`builds-in-subagents`, `needs-alone-order`), and SHALL NOT read a component's `paths` from the `flight` model.

#### Scenario: no interactive or waiting state
- GIVEN the web app's source
- WHEN the tests run
- THEN the implementation view's state order is `claimed`, `ready`, `blocked`, neither its state labels nor the graph's state colours hold `waiting`, and no source file under `web/src` names an `interactive` state, `waiting_on` or `Needs an interactive session`

#### Serves
- capability/see-project-state
- architecture/component-reconciliation

### Requirement: approve-button <!-- base: ff59f9d107673204 -->
Approve intent SHALL send one approve request per tap and none while one is in flight, and once the Worker accepts it SHALL read Approved and stay disabled. It SHALL be disabled offline, for a review without a fingerprint, and for a review that already shows an approval, as `approve-by-branch` says.

#### Scenario: one request, then approved
- GIVEN a spec branch's review, online, with a fingerprint and no approval
- WHEN Approve is tapped twice in quick succession and the Worker accepts the first request
- THEN exactly one approve request was sent, and the button reads Approved and is disabled

#### Scenario: disabled
- GIVEN a review with no fingerprint, a review that already shows an approval, and a review with a fingerprint shown offline
- WHEN each is shown
- THEN Approve is disabled in all three

#### Serves
- capability/follow-from-browser
- capability/approve-intent
- architecture/approval-policy

### Requirement: signed-approvals <!-- base: 9bdf647354848e06 -->
When a spec branch's review says `signed`, the PR view SHALL draw Approve disabled, never reading "Approved", with a note beside it that the repository counts only approvals signed with the reviewer's SSH key and the command `clm approve <branch>` for that branch; SHALL mark each approval in the header "signed" or "unsigned, doesn't count" from its `signed`; and SHALL say "no signed approval" when it lists approvals and none is signed. An approve answered 422 with `error` `signed` SHALL load the review again from the Worker. When the review says `signed` false or doesn't say, the view SHALL show neither the note nor the marks.

#### Serves
- capability/follow-from-browser
- architecture/signed-ledger-records

#### Needs
- web.worker.review/signed-repository

#### Scenario: signed repository
- GIVEN `spec/x`'s review with `signed` true, a fingerprint, and one approval by `clunny/web` with `signed` false
- WHEN the PR view is drawn online
- THEN Approve is disabled and doesn't read "Approved", the note shows `clm approve spec/x`, the header marks `clunny/web` "unsigned, doesn't count" and says "no signed approval", and tapping Approve sends no request

#### Scenario: signed approval
- GIVEN `spec/x`'s review with `signed` true and one approval by `dana/9f9f9f9f` with `signed` true
- WHEN the PR view is drawn
- THEN the header marks `dana/9f9f9f9f` "signed" and doesn't say "no signed approval"

#### Scenario: became signed
- GIVEN `spec/x`'s review drawn with `signed` false and fingerprint `f1`, and a Worker that answers the approve with 422 and `error` `signed`, and then the review with `signed` true
- WHEN the reviewer taps Approve
- THEN the review is requested again and the note shows `clm approve spec/x`

#### Scenario: unsigned repository
- GIVEN `spec/x`'s review with `signed` false, a fingerprint and no approvals, and another review with no `signed` field
- WHEN each PR view is drawn online
- THEN Approve is enabled and no note or approval mark shows

### Requirement: propose-from-backlog <!-- base: 9531d23b17c31b0d -->
The Backlog view SHALL offer New proposal: a form with a title and an intent, and no picker of components, that posts an object with `/title`, `/body` and `/nodes` `[]` to the Worker's proposals route once per tap and none while one is in flight, on success closes and asks for the implementation state again, on a 409 says the title is taken and keeps the form, and on a 422 shows the answer's reason. It SHALL list the implementation state's `proposals` under Proposed, between Waiting on you and the columns, each with its title, the person part of its proposer and its time and a note that the next build files it, with Withdraw on those whose `mine` is true, which sends one `DELETE` for the proposal's `id` and then asks for the implementation state again. Both SHALL be disabled offline.

#### Scenario: proposed
- GIVEN the Backlog of `o/a` online, and a Worker that answers the proposal with 200 and then lists it in `proposals` by `alice/web`
- WHEN the user opens New proposal, enters the title "Export CSV" and the intent "Let me export.", and submits twice in quick succession
- THEN exactly one request posted `/title` `"Export CSV"`, `/body` `"Let me export."` and `/nodes` `[]`, the form holds no picker of components, the form closed, and Proposed lists "Export CSV" with `alice` and the note

#### Scenario: title taken
- GIVEN a Worker that answers the proposal with 409
- WHEN the user submits
- THEN the form stays open saying the title is taken

#### Scenario: withdraw
- GIVEN Proposed listing `p1` by `alice/web` with `mine` true and `p2` by `bob/web` with `mine` false
- WHEN the view is shown and `p1`'s Withdraw is tapped
- THEN `p2` shows no Withdraw, one `DELETE` went to `/api/repos/o/a/proposals/p1`, and the implementation state was requested again

#### Serves
- capability/keep-a-backlog
- capability/follow-from-browser
- architecture/git-is-truth

#### Needs
- web.worker/proposals

### Requirement: comment-on-review <!-- base: 1279617c36f2004d -->
The PR view's Document tab SHALL carry a Comment control on the intent and on each requirement card that posts `{"fingerprint": <the review's fingerprint>, "target": <null for the intent, the requirement's ID for a card>, "text"}` to the Worker's comments route for the branch, once per tap and none while one is in flight, and loads the review again on success. It SHALL draw each entry of the review's `comments` under its target, the intent for a null target or a requirement the review shows no card for, with the person part of its writer and its time, marked "on an earlier version" when `current` is false, and each entry of `change_requests` whose `held` is false under the intent the same way, marked "since approved" when `current` is true. A review without the two fields SHALL be read as having none. Comment SHALL be disabled offline and when the review has no fingerprint.

#### Scenario: comment on a requirement
- GIVEN `spec/x`'s review with fingerprint `f1`, which modifies `n/r`, and a Worker that answers the comment with 200 and then the review with a comment on `n/r` by `alice/web` at `f1`
- WHEN the reviewer taps Comment on `n/r`'s card, enters "Name the rule" and submits
- THEN one request posted `{"fingerprint": "f1", "target": "n/r", "text": "Name the rule"}` to `/branches/spec/x/comments`, and `n/r`'s card shows the comment by `alice`

#### Scenario: earlier version
- GIVEN a review with fingerprint `f2` whose `comments` holds one at `f1` with `current` false and target `null`, and whose `change_requests` holds one at `f2` with `current` true and `held` false
- WHEN the Document tab is shown
- THEN the comment is drawn under the intent, marked "on an earlier version", and the request under it, marked "since approved"

#### Serves
- capability/approve-intent
- capability/follow-from-browser
- architecture/humans-review-intent
- architecture/approval-policy

#### Needs
- web.worker.review/review-feedback

### Requirement: request-changes <!-- base: 8ce19ec830958f6f -->
The PR view SHALL offer Request changes beside Approve, in the Approve bar on phones, which takes a text and posts `{"fingerprint": <the review's fingerprint>, "text"}` to the Worker's request-changes route for the branch, once per tap and none while one is in flight; on success it SHALL load the review again, and on a 409 say the branch changed since the review was shown and load the review again, as Approve does. The header SHALL show each entry of `change_requests` whose `held` is true as changes requested by the person part of its writer, with its time and text, and say the branch won't land until the author pushes a change or the requester approves. Request changes SHALL be disabled offline and when the review has no fingerprint, and enabled in a signed repository, after an approval and after an earlier request.

#### Scenario: requested
- GIVEN `spec/x`'s review with fingerprint `f1` and `signed` true, online, and a Worker that answers the request with 200 and then the review with a held request by `alice/web`
- WHEN the reviewer taps Request changes, enters "Split it" and submits twice in quick succession
- THEN exactly one request posted `{"fingerprint": "f1", "text": "Split it"}` to `/branches/spec/x/request-changes`, and the header says changes requested by `alice` with "Split it"

#### Scenario: branch moved
- GIVEN the review drawn with fingerprint `f1`, and a Worker that answers the request with 409 and the review with fingerprint `f2`
- WHEN the reviewer submits a request
- THEN the status says the branch changed since the review was shown, and the review on screen is the one with fingerprint `f2`

#### Scenario: disabled
- GIVEN a review with no fingerprint, and a review with a fingerprint shown offline
- WHEN each is shown
- THEN Request changes is disabled in both

#### Serves
- capability/approve-intent
- capability/follow-from-browser
- architecture/humans-review-intent
- architecture/approval-policy

#### Needs
- web.worker.review/review-feedback

### Requirement: answer-buttons <!-- base: c08a80318fd2fed1 -->
A `question` item with `choices` SHALL offer one button per choice, in their order, and a `human-task` item one button, Done, which sends `done`; no other item SHALL offer any. A tap SHALL post `/answer` to `POST .../backlog/{slug}/answer` once, and none while one is in flight, then ask for the implementation state again, so the answer shows from the response's `answers`. The button of an answer the person gave (an answer whose `mine` is true) SHALL be drawn chosen, and tapping another SHALL replace it. A 422 or a 404 SHALL show why under the buttons and keep them enabled. The buttons SHALL be disabled offline.

#### Scenario: answered
- GIVEN item `q` with choices `yes` and `no`, online, and a Worker that answers 200 and then lists the answer `yes` by `alice/web` with `mine` true
- WHEN `yes` is tapped twice in quick succession
- THEN exactly one request posted `/answer` `"yes"` to `/api/repos/o/a/backlog/q/answer`, the implementation state was requested again, and `yes` is drawn chosen

#### Scenario: done
- GIVEN `human-task` item `h`
- WHEN its page is shown and Done is tapped
- THEN one request posted `/answer` `"done"` to `h`'s route

#### Scenario: offline
- GIVEN item `q` with choices, offline
- WHEN its page is shown
- THEN its buttons are disabled

#### Serves
- capability/follow-from-browser
- architecture/git-is-truth

#### Needs
- web.worker/answers
