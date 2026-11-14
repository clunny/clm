---
component: web
---

## MODIFIED Requirements

### Requirement: approve-by-branch <!-- base: 971e758a2a1119e1 -->
Approve SHALL post `{"fingerprint": <f>}`, where `<f>` is the fingerprint of the review on screen, to the Worker's approve route for that branch, and on success SHALL load the review again and show the approval from its approvals, not from a label. On a 409 it SHALL say the branch changed since the review was shown, show no approval, and load the review again from the Worker, never from the copy kept for offline use, drawing it with that message still shown, so that the next Approve pins what is then on screen (`approve-pins-the-review`); on any other failure it SHALL say "Not approved" with the status. Approve SHALL be disabled offline, when the review has no fingerprint, and once the review lists an approval. It acts on the branch review `spec-branch-review` loads, so the two build together.

#### Scenario: approve
- GIVEN `spec/x`'s review with fingerprint `f1` and no approvals, and a Worker that answers the approve with 200 and then the review with an approval by `clunny/web`
- WHEN the reviewer taps Approve
- THEN one request posts `{"fingerprint": "f1"}` to `/branches/spec/x/approve`, the review is requested again, the header names `clunny/web`, and Approve is disabled

#### Scenario: no fingerprint
- GIVEN a review with no fingerprint
- WHEN it is shown
- THEN Approve is disabled

#### Scenario: branch moved
- GIVEN `spec/x`'s review drawn with fingerprint `f1`, and a Worker that answers that approve with 409 and the review with fingerprint `f2`
- WHEN the reviewer taps Approve
- THEN the status says the branch changed since the review was shown, no approval is shown, and the review on screen is the one with fingerprint `f2`

#### Serves
- capability/follow-from-browser
- capability/approve-intent
- architecture/approval-policy
- architecture/git-is-truth

#### Needs
- web.worker.review/approve

### Requirement: answer-from-app <!-- base: 074f7d4cc26bae1d -->
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
- architecture/git-is-truth

#### Needs
- web.views/answer-buttons
- web.views/home-waiting-on-you
- web.views/backlog-item-page
- web.worker/answers
