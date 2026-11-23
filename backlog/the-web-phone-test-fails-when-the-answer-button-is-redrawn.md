---
title: "The web phone test fails when the answer button is redrawn during its scroll"
kind: bug
components: [web]
---

`clm land --check-only` on impl/view.repo (a branch that changes no web file) failed once in `web/e2e/app.spec.ts:40` ("phone › Home on a phone: no sideways scroll, and a question's answer buttons can be tapped"): `locator.scrollIntoViewIfNeeded: Element is not attached to the DOM` at line 46, after `expect(yes).toBeVisible()` passed. The page redrew `button.answer[data-answer=yes]` between the visibility check and the scroll, so the test is timing-dependent. Run again with nothing changed, the whole check passed. A flaky test fails unrelated lands and costs each a full re-run of the check (~6 minutes). The fix probably lies in the test: wait for the page to settle (or retry the scroll on a fresh locator) before scrolling.
