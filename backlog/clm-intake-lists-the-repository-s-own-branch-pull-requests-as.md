---
title: "clm intake lists the repository's own branch pull requests as contributions"
kind: bug
components: [orchestrator.triage]
---

Step: `/clm:triage` step 1 and 2a, `clm intake` followed by the *Contributions* choice.

What happened: `clm intake` printed `pull <n> <author>` lines for the repository's own `impl/*` build branches' pull requests (opened by `clm pr open`, head in the same repository, not a fork). The triage skill sends every `pull <n>` line through the issue-verifier and the `contribution` skill, which recommends closing any pull request touching code. The verifier described these same-repository pull requests as coming "from a fork". Following the skill would have closed in-flight build pull requests; by the time they were reached they had already been landed by `clm land`.

Expected: `clm intake` lists only pull requests whose head is in another repository (GitHub's `isCrossRepository`), since every branch CLM pushes opens a pull request for reading its diff (`clm pr open`), and those are not outside contributions (`outside-contributions-are-intent`).

Rule involved: `orchestrator.triage`'s `clm intake` requirement ("print one line for each ... `pull <n> <author>` for a pull request") and `host/list-contributions`, which return every open pull request. Needs a spec change, since the requirement as written lists them all.
