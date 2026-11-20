---
title: "`bounded-runs` asks `run_within` to know at exit whether a pipe will close before the limit"
kind: question
components: [vcs]
choices:
  - "Reword the first sentence to the Design's 100 ms"
  - "Wait for the streams up to the limit, as run does"
---

`vcs/bounded-runs` (`44e9c483`) says `run_within` SHALL behave as `run` when the command exits within `limit` and its streams have closed once `limit` has passed, and SHALL return within one second after the command exits when a process it started still holds a stream once `limit` has passed. At the moment the command exits the call can't tell which of the two it is in: a process the command started that writes for two seconds and then closes the pipe, well within a 10-second limit, falls under the first sentence (`run` would return its bytes), but waiting for it breaks the one-second bound of the second whenever that process holds the pipe past the limit instead.

The Design settles it as the change `20261119-run-within-can-report` chose: the call waits at most 100 ms after the command exits and returns the command's outcome with what the streams sent by then, so a process that writes later than that loses its later bytes, and a reader thread not scheduled within 100 ms on a loaded host can return fewer bytes than the command wrote. The build follows the Design. Both reviewers of the `impl/vcs` build (code review and spec audit) read the first sentence as broken by it.

A spec change should make the two agree: reword the first sentence to say what is returned when a stream is still open 100 ms after the command exits, or state a different wait.
