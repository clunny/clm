---
title: "Ctrl-C during clm verify's filing still kills the git and land commands it runs"
kind: bug
components: [verify, clm]
---

`clm/verify-command` says that from the `verify passed ` line until `clm-verify`'s call returns, the mutation pass and the filing together, `SIGINT` and `SIGTERM` set the stop request instead of ending the process. Only `clm`'s own process catches them. A Ctrl-C at the terminal goes to the whole foreground process group, so the `git` commands and the land that `verify/files-unproven`'s filing starts are interrupted directly. The filing can stop half-way (for example, its branch pushed but not landed) while `clm verify` still exits 130 as if the stop were orderly.

Expected: the filing's child commands are not ended by a signal sent to the terminal's process group while the window holds, for example by running them in their own process group, or the requirement says what a Ctrl-C does to them. The mutation pass already runs its command in a group of its own. Found by the review of wave 3's root build.
