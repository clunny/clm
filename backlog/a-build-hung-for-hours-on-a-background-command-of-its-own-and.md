---
title: "A build hung for hours on a background command of its own and nothing noticed"
kind: bug
components: [plugin.build]
---

What happened: during /clm:build, one build subagent started a background Bash command whose first part read standard input (a stray `cat >> <file>` ahead of the real edit and test run). The command waited on input for over five hours. The subagent was marked as waiting on its own background work, so the orchestrating session, following the build skill's `background-work-stopped` rule, counted it as still running and did nothing: no `clm run --finish`, no retry, no word to the human. Only when the human asked whether it was stuck did the session look, find the blocked process and, with the human's yes, kill it, after which the build finished and landed.

Expected: the process should bound or notice a build that waits on its own command for far longer than any land or test run takes. For example, the build prompt could tell builds that background commands must never read standard input and should run under a timeout, and the build skill could tell the orchestrating session to look at a build that has pushed nothing and stayed waiting past some bound, and tell the human, rather than wait indefinitely.

Rules involved: `background-work-stopped` (build skill step 3) and the `clm run` build prompt's guidance on running commands.
