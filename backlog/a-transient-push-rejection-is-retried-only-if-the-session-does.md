---
title: "A transient push rejection is retried only if the session remembers to"
kind: idea
components: [host]
---

The build prompt tells a session to retry `clm branch push` at most twice when the remote rejects it for a transient reason, and never to retry a hook's decline or a lease rejection. The requirement that pinned those sentences, `orchestrator.run/prompt-commands`, was a scan of prompt text and is deleted in `20261113-phase-2-delete`.

`clm branch push` can tell the two kinds of rejection apart from git's output, so it could make the bounded retry itself and the prompt would need no rule.
