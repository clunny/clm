---
title: "clm branch delete refuses an abbreviated --lease as a tip this clone didn't push"
kind: bug
components: [host]
---

Step: releasing a claim with `clm branch delete <branch> --lease <sha>`, given an abbreviated SHA of the branch's actual tip, pushed from this clone.
What happened: it exited 1 saying this clone didn't push the branch at that SHA, and deleted nothing. The check compares the reflog's full hash with the lease as text, so a short form never matches.
Expected: `branch-delete` deletes when the tip is `<sha>`; an abbreviation naming that tip should be resolved to the full hash first. The refusal's reason was false and sent the session reading the source to find out why.
