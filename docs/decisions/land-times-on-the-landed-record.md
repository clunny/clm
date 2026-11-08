---
title: "A land's check times go on its landed record, not on the checked record"
date: 2026-11-08
status: accepted
---

When `clm land` runs `[land] check`, it records the command's time and each marked step's time and selecting files on the `landed` record it appends. `clm land --check-only` and `clm verify` record no times, and the `checked` record stays as `graph-test-selection` keeps it: a fact about a code key.

**Why.** The times describe one land's run, and what that run did depends on the land's selection, not on its code key. Two trees with one code key hold the same code but can run different tests: a full `--check-only` run, a land's selected run, a land that runs only the readers of checked code. Keying times by code key would credit a spec land, or a land after a build's `--check-only`, with another run's times, and "how many recent lands went over the target" would count runs that weren't lands. On the `landed` record, a land is *timed* exactly when it ran the check, which is what the target is about.

**Alternatives.**
- *Times on `checked`, joined to lands by code key.* Rejected for the reason above, and because `graph-test-selection` already keeps what ran off that record.
- *Times on `checked`, joined only to records a land wrote.* Exact, but it needs a second field to say who wrote the record, and a readers-only land, which appends no `checked` record, would still have nowhere to put its times.
- *A local file under `.clm/`.* Lost to other clones and to `clm health` run elsewhere. The ledger is already the shared, per-writer record of lands (`per-writer-ledgers`), so the fields cost no new write and no contention.
