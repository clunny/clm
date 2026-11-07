---
title: "Triage skill doesn't say what to do with clm intake's issue imported lines"
kind: bug
components: [plugin]
---

Step: /clm:triage, step 1 and step 2a (Contributions).

What happened: `clm intake` printed an `issue <n> imported <slug>...` line, a form `orchestrator.triage`'s `clm intake` requirement states (an open issue an item on origin/main's backlog already names in its `issues`). The triage skill's step 1 lists the line forms `clm intake` prints (`issue <n> new`, `issue <n> resolved`, `pull`, `alert <n> new`, `alert <n> resolved`, `alerts unavailable`) but not `issue <n> imported` or `alert <n> imported`, and step 2a gives no handling for them, nor says whether they count toward the Contributions choice on the menu. The session had to read the spec to learn the line means "already taken in, nothing to do".

Expected: the skill names the `imported` forms in step 1, says they need no action (the issue stays open until its item is resolved, when it shows as `resolved`), and either leaves them out of the Contributions count or says they are listed for information only.

Rule: the skill should state each line form the command it runs prints (`method-in-the-plugin`).
