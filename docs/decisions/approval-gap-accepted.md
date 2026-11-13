---
title: "The guard prevents accidents; the gap to a hard approval boundary is accepted in writing"
date: 2026-11-13
status: accepted
---

Only a human approves a change that needs review (`approval-policy`), and only `clm land` pushes `main` (`land-locally`). What holds a session to that is the agent harness's guard, which makes the human confirm `clm approve` and blocks a push to `main`. It is not a hard boundary: it rests on the harness's own confirmation prompt, an agent CLM doesn't recognise runs no guard and looks like the human's terminal, and a session set on getting round the guard has the same shell the human has.

Signed approvals would not close the gap here, so they stay off in this repository. An ordinary key on disk is as reachable to a session as to the human; a touch key would make every unattended land wait for a hand; and a signed repository loses approval from the web app (`web-approve-off-when-signed`).

So the gap is accepted in writing: the guard prevents accidents, and `clm ledger verify` audits after the fact, finding an approval or a `main` commit that no record explains. The capability `guard-sessions` and the `#### Untested` blocks of `approval-policy` and `land-locally` say so. A separate touch key for approvals alone, with an answer for the web app, is left for when others join the project.
