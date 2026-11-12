---
title: "A failed local verify records no log of which tests failed"
kind: bug
components: [verify]
---

Step: `/clm:triage` step 2a, putting a `verify <sha> new <culprit> <component>` line from `clm intake` to the human.

What happened: the triage skill tells the session to show the line and, when taken in, to write a body naming "the verify log". The ledger's `verified` record for the red commit holds `passed: false`, the culprit, its components, `mode: local` and `run: null`, and nothing else: no log path, no failing test names, no output excerpt. The session could not say which test failed, so it could not judge whether the culprit (a change touching only a retry loop) was real or a flaky probe, and the human had to decide on the bisect alone.

Expected: a failed `clm verify` in `local` mode keeps the failing check's output (or at least the failing tests' names) somewhere the intake line, `/clm:verify` and the triage skill can point at, so the human's take-in decision (`verify-after-land`) rests on the failure itself.
