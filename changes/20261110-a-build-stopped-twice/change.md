---
status: applied
from: backlog/a-build-stopped-twice-by-the-same-outside-cause-is-rebuilt
resolves:
  - a-build-stopped-twice-by-the-same-outside-cause-is-rebuilt
---

## Intent

Let `/clm:build` resume a build stopped by an outside cause (a usage or rate limit, a full disk, a network outage, a tool or host failure) each time that cause stops it and clears, instead of building it again on its second such stop. "At most once for the same reason" applies only to a stop whose cause could lie in the work itself, such as a check or test that keeps failing. When the same outside cause stops builds again, `build` tells the human it recurred and acts on the cause where it can: when parallel builds and their land checks fill the disk or reach a limit, it resumes the stopped builds one at a time and launches no new build until they have returned.

## Problem

In `/clm:build`, six builds stopped on a full disk, were resumed once space was freed, and stopped again because their land checks, running in parallel, refilled the disk. `build-resumes-sound-builds` says a build is resumed at most once for the same reason and its second such stop is the retry's first, so the next step was a fresh build of each on the session's own model. The work was sound and the cause was outside it, so a fresh build would meet the same disk and only repeat the work. The human overrode the rule; the builds were resumed one at a time and all six landed. The requirement as written states the once-only limit, so the fix changes `specs/` (`code-fix-needs-a-written-rule`): it modifies that requirement and `plugin.build`'s Design, and amends the decision `resume-sound-builds`.

## Approaches

1. **Outside causes resume every time they clear; the once-only limit stays for causes that could lie in the work; a recurring outside cause is acted on, not rebuilt.** Chosen. It matches what the human did, keeps a bound where a fresh context can help, and reuses the resume path as it stands (`reuse-first`).
2. **A higher count for outside causes** (resume up to two or three times). Rejected: any count is a number the cause doesn't respect, and past it every rebuild repeats sound work for nothing.
3. **Keep the rule and only tell the session to run fewer builds after the first stop.** Rejected: it still rebuilds sound work on the second stop, which is the bug.

## Outcome

`skills/build/SKILL.md`'s resume rule distinguishes the two kinds of stop: an outside cause resumes each time it clears; a cause that could lie in the work resumes at most once, its second stop the retry's first. A recurring outside cause makes the session say so to the human and, when parallel builds bring it back, resume the stopped builds one at a time and launch nothing new until they have returned. The test of `build-resumes-sound-builds` also requires `one at a time`.

## Constraints

- The judgement stays the session's, from the `stopped:` reason and what it knows of the machine (`resume-sound-builds`); no list of reasons in `clm`.
- A cause only the human can clear, such as a full disk, is still resumed only once the human says it has cleared, which bounds the loop.

## Edge cases and workarounds

- A cause the session misjudges as outside the work makes the build stop again for it; each resume then waits for the cause to clear, so the human, who confirms a full disk cleared, sees the recurrence and can say to build it again. Accepted, as `resume-sound-builds` already accepts a misjudged cause.

## Open questions

None.
