---
title: "Removing a capability is built like removing a requirement: the change lists it, and the build removes its tests"
date: 2026-11-13
status: accepted
---

A capability is cited by its tests like a rule (`human-layer-proof`), but it is removed by a hand edit of `CAPABILITIES.md`, not by a delta. So `removed-requirements-build-out`, which reads a removal from a delta's `REMOVED Requirements`, never saw it: the tests' markers dangled, `clm check` failed the spec branch, and the branch couldn't delete the tests without becoming `mixed`. `20261113-human-layer-rewrite` got two entries out only by landing a code branch first that deleted their tests, which left the capabilities unproven on `main` before the human had agreed to remove them (`backlog/a-spec-branch-can-t-remove-a-capability-entry-a-test-cites`).

The change whose branch removes the entry now says so in `change.md`: `removes_capabilities: [<slug>, ...]`. While that change is outside the archive and `CAPABILITIES.md` holds no entry of the slug, the capability is *removed* as a requirement a delta removed is: its markers are `obsolete`, not dangling, `clm pending` lists them on the component that builds the capability tests, whose next build deletes the tests (`no-dead-code`), and the change isn't complete, so isn't archived, while a marker is left. Archiving turns the exemption off, as for a requirement.

Nothing else changes. The entry is still removed by hand, the branch still needs the human's yes because it edits `CAPABILITIES.md` (`removal-needs-a-yes`), and an architecture entry needs none of this, since no test cites one.

The key only declares. A slug that `CAPABILITIES.md` still holds, or never held, removes nothing and is no violation. A mistyped marker could be hidden by listing its slug, but only until the change is archived, and the change can't complete while the marker is there, so the build removes it either way.

**Known limit: no order between the capability's build and its rules' builds.** A change that removes a capability often removes or changes the rules behind it. Until the `capability` build has deleted the obsolete tests, they still run at every land, so a rule's build that takes the behaviour away first fails them, and it can't delete them, since they are `capability`'s files (`path-ownership`). Nothing orders the two builds. The obsolete line is never blocked and carries its change's priority, so `clm next` offers the `capability` build at once, and a rule's build that meets the failure stops at that requirement and builds it once the tests are gone. Making the change's rules wait on the obsolete line would close this, as a new kind of wait in the pending list and the queue; it is left until a removal meets it (`yagni`).

Rejected:
- **A delta for the `capability` component with `REMOVED Requirements`.** It would reuse the reading of deltas, but `clm apply` would then have to edit `CAPABILITIES.md`, a file people edit by hand, and `clm rebase`'s regeneration, the stale-base check and the land's comparison of `specs/` would all have to learn a second file. A project with no `capability` component would have no component to write the delta for.
- **Reading the removal from git**, by comparing `CAPABILITIES.md` with `origin/main`. `clm check` reads one tree, and once the branch lands there is nothing left to compare, so the markers would dangle on `main`.
- **Treating every marker on a missing capability as obsolete.** A mistyped marker would never be reported.
- **Letting the spec branch delete the tests.** That makes spec branches mixed, as `removed-requirements-build-out` already rejected.
- **Keeping the workaround**, a code branch first. It removes proof before the human has agreed to the removal, and needs two lands in the right order.
