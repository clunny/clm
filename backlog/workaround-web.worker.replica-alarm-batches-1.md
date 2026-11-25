---
title: "A lapse alarm recomputes before the claim reads lapsed"
kind: workaround
components: [web.worker.replica]
requirement: web.worker.replica/alarm-batches@9c14a20a
---

*A claim lapses while a page is open* states that the alarm at 14:00 drops the claim and posts one notice. The object fires the alarm at the time `replica`'s `recompute` gave (the claim's tip time plus the expiry), as the requirement says, but `clm_orchestrator::claim_state` reads the claim live at that instant and lapsed one second later. So the alarm at 14:00 recomputes with the claim still listed, posts a notice, and re-arms for 14:00:01, when a second recompute drops it and posts a second notice. The test runs the alarms to 14:00:01 and asserts that an alarm fired at 14:00, the claim is gone after, and each notice names 32 hex digits no event has; it can't assert one notice. The fix lies in `replica`/`orchestrator`, filed as backlog/a-claim-reads-live-at-the-instant-recompute-says-it-lapses.
