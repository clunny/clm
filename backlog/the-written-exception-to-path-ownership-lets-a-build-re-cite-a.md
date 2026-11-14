---
title: "The written exception to path-ownership lets a build re-cite a test in another component's file but not remove an obsolete one"
kind: bug
components: [orchestrator.run]
origin: 20261113-phase-2-delete
---

Step: the builder check of a spec change that removes requirements whose tests sit in files another component's `paths` hold.

What happened: `clm pending <component>` lists each obsolete marker on the component whose requirement was removed, with the file it sits in, and `removed-requirements-build-out` says that component's next build removes the code and tests the marker marks. The guide's list of exceptions to `path-ownership` (`clm guide how-it-works`) names only re-citing or updating a test in a file no component's `tests` match. It names no exception for deleting such a test, or for cutting one that mixes kept and removed assertions. A build that reads the exception list strictly leaves the requirement pending and files a question.

Expected: the exception list and the build-out rule agree, so a build knows it may remove a test that cites its own removed requirement wherever the file sits, or the pending list places such a marker on the component that owns the file.

Rule or decision involved: `path-ownership`, `removed-requirements-build-out`, `stale-markers-keep-behind`.
