---
title: "The fold's prompt doesn't carry a branch's cuts to the approval ask"
kind: deferred
components: [orchestrator.triage]
---

`clm-triage`'s fold prompt has a fold report `proposed <batch> review <url>` for a folded branch that needs review, and the triage skill then asks the human for one approval. Since `gate/cuts`, a branch may need review because it cuts something, and the `gate/cut` lines are what the human reads. The triage skill now says to show them, read from the pull request's body. The fold's prompt should tell the fold to keep a scenario's name, to declare deleted tests in `deleted-tests.txt`, and to name the cut lines in what it hands back, as the propose skill now says. It is the fixed text of a prompt (`prompt-text-is-source`), so the fix is code only, under `clm claim orchestrator.triage --fix backlog/the-fold-prompt-doesn-t-carry-a-branch-s-cuts-to-the-approval-ask`, with no test. The draft's prompt, in the same crate, should say the same of a draft's change.
