---
title: "A merged requirement's sentences are counted with those of the requirements that took its scenarios"
date: 2026-11-22
status: accepted
---

`cuts-are-what-a-program-lists` reads a requirement whose every scenario moves to another requirement as merged, not gone, and counts `SHALL` sentences only for a requirement both sides hold. So a merged requirement lost every sentence it stated with no line: in `20261122-wave-4-orchestrator-by-the-rule`, `orchestrator.run/says-component` and `orchestrator.triage/says-component` each moved their one scenario into another requirement and took with them the promise that the prompts and messages use no retired word, and the gate listed neither. The change named them by hand, as wave 3 had for `sync/agents-md-spec-path`, which is the session's word the gate exists to replace.

**The decision** (`gate/cuts`). A *merge* is a merged requirement together with every requirement that took one of its scenarios, joined with any other merged requirement those took from. Its stating sentences are counted together on each side, a requirement a side doesn't hold counting none there, and a merge that states fewer at the head than at the base is the cut `<ids> merged into <ids> state <n> sentences where they stated <m>`, which makes the branch need review. A requirement both sides hold that took scenarios is still compared on its own as well, so a loss of its own sentences keeps its *shortened requirement* line, which the earlier rules versions asked review for. Under the gate's rules version 8 the cut needs review, and versions 1 to 7 keep their answer (`gate/rules-version`, `audit-by-the-rules-in-force`).

**Why counted whole.** Two requirements merged into a third would each look kept against the third's count, since each is compared with the same sentences. Counting the merge together asks the one question that matters: do the requirements left state as many rules as the ones that went into them.

**What stays a reader's.** As for a shortened requirement: a merge that keeps its count while stating something else, which the human reads only when the change names it.

Considered:
- **A line for every merge, `requirement <id> is merged into <id>`, asking review whatever the count.** Rejected: a merge that carries its sentences over is the reshaping a slimming pass does most, and its review would read a line that drops nothing, while a reword that keeps the count of a requirement both sides hold already lands on checks.
- **Counting each merged requirement against its takers alone.** Rejected: two requirements merged into a third with the third stating one of their two sentences give no line, as above.
