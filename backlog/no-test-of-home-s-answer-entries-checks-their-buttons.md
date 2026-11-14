---
title: "No test of Home's Answer entries checks their buttons"
kind: bug
components: [web.views]
origin: 20261113-phase-2-delete
---

`web.views/home-waiting-on-you` says Home's Answer section lists each item with its buttons. Its tests in `web/src/views/home.test.ts` call `renderHome` without `answering`, so no button is drawn and only the links are asserted. The buttons are proven only under `web.views/answer-buttons`.

The requirement was flagged while `web.worker/answers` was unbuilt (`workaround-web.views-home-waiting-on-you-1`, deleted in `20261113-phase-2-delete`). It is built now: pass `answering` in the scenario test and assert the choices of a `question` item, `Done` on a `human-task`, and no button on an entry answered in a session. `HomeInput.answering` is optional only for the tests that leave it out.
