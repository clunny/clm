---
title: "The spec-slimmer's prompt predates what was settled after wave 1"
kind: deferred
components: [plugin.agents]
---

`plugin/agents/spec-slimmer.md` is launched once per component by each wave of phase 5. `requirements-state-what-the-contract-shows` was amended on 2026-11-20, and the prompt doesn't say what it now holds:

- under **Newly stated**: one scenario for a clause, with its cases listed inside it, and none that shows what another scenario of the requirement already shows;
- no clause for input that can't reach the code, such as a reading the system never gives; the entry says why it can't;
- of each clause written from the code, whether the code or the intent is right, with the entry saying which it took and why.

The prompt is source (`prompt-text-is-source`), so this is a code fix under `clm claim plugin.agents --fix`, with no test, due before wave 2 launches the slimmer.
