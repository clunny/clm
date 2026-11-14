---
status: done
resolves: []
---

## Intent

Set the backlog item `the-written-exception-to-path-ownership-lets-a-build-re-cite-a` from `kind: bug` to `kind: deferred`, with one sentence saying what it waits on, and design nothing. No spec, code, guide or skill text changes.

The item is not fixed on `main`: the guide's list of exceptions to `path-ownership` (`docs/guide/how-it-works.md`, "What a build may change") still names re-citing a test in a file no component's `tests` match, and not removing one.

The plan in force, `backlog/a-passing-check-proves-less-than-the-spec-claims`, already holds the answer. Its phase 2 notes leave this item to phase 4 by name, and its open questions say: "Whether a build may remove a test in a file another component owns. Phase 2's builds do, by `removed-requirements-build-out`; the guide's exception list doesn't say so. Phase 4 removes the question." Phase 4 gives each crate its own tests and retires `path-ownership`'s transitional exceptions, so the list this item asks to extend goes away. The freeze is on, a defect's default answer under it is to accept, and an exception to it is the human's to make, never a session's. So this draft adds no exception to a list the plan is about to delete.

Decided work left for later is what `kind: deferred` is for. As a `bug` the item is drafted again by every triage, each draft a whole session that ends where this one did. As `deferred` it prints as a line in `clm triage` until the human judges it due, and phase 4's change resolves it. It stays out of `resolves`, since nothing here fixes it, and it stays on disk, since the plan cites it by path.

Builds are not held meanwhile. What a build reads is its prompt, not the guide, and the prompt (`crates/clm-run/src/lib.rs`) says for each obsolete requirement "A live change removed it; remove the code and tests marked at" the listed locations, which name the file wherever it sits. `changes/20261113-phase-2-delete/change.md` says the same under "For the builds", and its `deleted-tests.txt` is the approved list of what goes. What stays wrong until phase 4 is one bullet of a guide page.
