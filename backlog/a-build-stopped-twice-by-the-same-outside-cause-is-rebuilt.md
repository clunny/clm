---
title: "A build stopped twice by the same outside cause is rebuilt instead of resumed"
kind: bug
components: [plugin.build]
---

What happened: in /clm:build, a build stopped because the disk was full. It was resumed once space was freed, and stopped again because parallel land checks refilled the disk. The build skill's rule (`resume-sound-builds`) says a build is resumed at most once for the same reason, and its second such stop counts as the retry's first, so the next step would be a fresh build on the session's own model. The human overrode that: the work was sound and the cause was outside it, so starting again gains nothing and only repeats the work. All six builds were then resumed again and landed once space was freed and they ran one at a time.

Expected: an outside cause that recurs, such as a full disk, a usage limit or a network outage, keeps the build resumable for as long as the cause is outside the work and clears. The "at most once" limit should apply only to stops whose cause could lie in the work itself, such as a check that keeps failing. A stop that recurs from an outside cause should prompt the orchestrator to fix that cause, for example by running fewer builds at once, rather than to rebuild.
