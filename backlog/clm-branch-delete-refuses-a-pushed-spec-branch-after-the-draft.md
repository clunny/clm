---
title: "clm branch delete refuses a pushed spec branch after the draft prompt deletes it locally first"
kind: bug
components: [host]
---

Step: a draft run found nothing left to design and followed the draft prompt's cleanup: delete the local spec/<id> branch, then remove its copy on origin with `clm branch delete spec/<id>`.
What happened: `clm branch delete` refused with "this clone didn't push origin/spec/<id> at <sha>; left it", although the same clone had pushed the branch with `clm branch push`. Deleting the local branch first seems to erase the record `clm branch delete` checks. The stub spec branch is left on origin.
Expected: the order the draft prompt gives (local first, then origin) removes both; or the prompt orders the steps so the check still holds; or `clm branch delete` recognises a branch this clone pushed after its local copy is gone.
Rule: the draft prompt's cleanup instruction and `clm branch delete`'s ownership check disagree.
