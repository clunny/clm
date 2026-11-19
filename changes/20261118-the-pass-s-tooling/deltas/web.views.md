---
component: web.views
base: {  }
---

## MODIFIED Requirements

### Requirement: spec-branch-review <!-- base: a7d7ac89960f64a8 -->
The PR list SHALL list the `spec/*` branches the Worker's branch list gives, each by name with its page's number and title when it has a page, and SHALL open each branch's review, routed by branch name and loaded from the Worker's branch review. The review's header SHALL show "needs review" when the review's top-level `review` is true, whether or not it has a `change`, and nothing for it otherwise, the writer and time of each approval the review lists, or "not approved" when it lists none, "checked" or "not checked", and a link to the page when there is one, and SHALL show no labels. Under the header, above the review's components, it SHALL list each line of the review's `cuts` as the Worker gives it, under the heading `Cuts`, and show no such heading for a review with none, so a person approving in the app reads what the branch removes before they tap Approve (`removal-needs-a-yes`). The PR list SHALL list nothing but the spec branches: no lands to review. The app SHALL request no `/pulls` route of the Worker.

#### Serves
- capability/follow-from-browser
- capability/approve-intent

#### Needs
- web.worker.review/spec-branches
- web.worker.review/pr-review

#### Scenario: branch without a page
- GIVEN the branch list holds `spec/a` with page #7 "Add a" and `spec/b` with no page
- WHEN the PR list is shown and `spec/b` is opened
- THEN both are listed, `spec/a` with #7 and "Add a", and `spec/b`'s review is loaded from `/branches/spec/b/review` and shows no page link

#### Scenario: approved by
- GIVEN `spec/x`'s review lists an approval by `clunny/web` and says `checked` false
- WHEN its review is shown
- THEN the header names `clunny/web` with the approval's time and says "not checked", and no request went to a `/pulls` route

#### Scenario: needs review without a change
- GIVEN `spec/arch`'s review with `review` true and no `change`, and `spec/plain`'s with `review` false and a `change`
- WHEN each review is shown
- THEN `spec/arch`'s header shows "needs review" and `spec/plain`'s doesn't

#### Scenario: a branch that cuts
- GIVEN `spec/x`'s review whose `cuts` hold `specs/n/SPEC.md: gate/cut: requirement n/r is gone`, and `spec/plain`'s with empty `cuts`
- WHEN each review is shown
- THEN `spec/x`'s shows `Cuts` with that line above its components, and `spec/plain`'s shows no `Cuts` heading
