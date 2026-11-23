---
title: "A root that doesn't exist is reported as origin that can't be reached"
kind: bug
components: [vcs]
---

`clm triage --root /nonexistent/x`, and each form of `clm draft` given such a root, print `origin can't be reached`. `clm-git`'s `fetch_retrying` reads a `git` process that fails to start, as it does in a directory that doesn't exist, as an offline `origin` (`FetchError::Offline`, `crates/clm-git/src/lib.rs`). The exit, 2, is right; the message sends the person to their network when the fault is the path they gave. Expected: a fetch that can't start `git` in its directory says so, naming the directory, and only a fetch whose `git` ran and couldn't reach the remote says `origin can't be reached`. Found by `orchestrator.triage`'s reader in wave 4 of `backlog/a-passing-check-proves-less-than-the-spec-claims`.
