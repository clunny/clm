---
title: "the auditor found a scenario the code can't observe: available, not free"
kind: workaround
components: [machine]
requirement: machine/machine-report@53f8492b
---

The spec-auditor found that scenario "available, not free" can't be observed: Readings holds no free-memory field, and which sysinfo figure read_system takes (available_memory, not free_memory) is seen by no test. The test judges 6 / 2.5 / 1 GiB available of 16 GiB; the 0.5 GiB free of the GIVEN has no input to go in.
