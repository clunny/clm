---
title: "clm defect files nowhere when the project's host is Azure DevOps"
kind: bug
components: [plugin.hooks, host]
---

With `defects = "ask"`, `clm defect` files through the project's host adapter, and on `host = "azure"` (as on `git` and `none`) `issue` prints the report and says there is nowhere to file it. But defect reports are meant for CLM's own repository on GitHub, whatever the project's host. A session on an Azure DevOps project reported ten defects this way and had to keep them in a file by hand.

Expected: `clm defect` files to CLM's repository whatever the project's host (through `gh` when it's available and signed in), and when it can't, writes the report somewhere that lasts (a backlog item or a file under `.clm/`) and names it, rather than only printing it.
