---
title: "The gate refuses a build's change to a file that isn't its to change, once the root names its wiring"
date: 2026-11-14
status: accepted
---

`path-ownership` was kept by the build prompt (`path-ownership-by-prompt`), and a sentence in a prompt enforces nothing (`three-kinds-of-rule`). That decision weighed a gate rule and set it aside for its edge cases. Most are gone: builds no longer make compile-only fixes in other components, and the commits `clm rebase` adds touch only spec paths.

For a branch named `impl/<component>`, each changed code file is refused that is none of: the component's own; a file no component owns; a file the root's `wiring` globs name; a file that cites the component (`req:<component>/` at the merge base or at the head); a capability test, edited and not added or deleted, citing a capability the component serves. The gate reads no branch name (`spec-branches-hold-spec-paths`), so it offers the list as a library call and `clm land`, which knows the branch, refuses; `clm land --check-only` says so first.

**The switch.** The rule is in force once the root sets `wiring`, even to `[]`, as `check`'s path ownership is in force once it sets `outside`. A project that was green before it upgraded stays green, and adopts the rule by naming its wiring.

**Why "a file that cites the component".** Of the last 429 builds here, 180 edited a test file another component owns that cites the building component: Cargo gives the binary only to the root package's tests, so 36 files under `crates/clm/tests/` prove other components, and 18 more sit in a parent's or a sibling's crate. The gate already reads where a test belongs this way for a code fix (`fixes-count-where-cited`). The alternative was an exemption per owner: eleven entries, each opening its files to every build and each needing review. The clause opens a file only to the builds it cites, covers adding and deleting as well as editing, which settles `backlog/the-written-exception-to-path-ownership-lets-a-build-re-cite-a`, and goes when each crate proves its requirements through its own entry point (`backlog/clm-s-cli-tests-prove-other-components-through-the-binary`).

**What it leaves to judgement.** That a wiring edit adds only the build's own entry, and that a capability-test edit keeps what the capability's scenarios state. A branch named outside `impl/` has no component and is not judged, so the small-change route is not held to it.

With that clause and this repository's wiring, 29 of those 429 builds (7%) would have been refused: most changed a root test file that cites another component, as when a change to the ledger's records broke `land`'s CLI tests, and the rest a parent's shared test helper, a child's source file or another component's code. Those are the coupled changes `expand-contract` builds in steps. One more build was `init` writing `upgrade`'s migrations alone, the written exception `template-change-ships-its-migration`. The rule has no clause for it: before this repository sets `wiring`, a template and its migration must share a component (`backlog/a-template-and-its-migration-live-in-different-components`), or a change must give the rule a way to keep that exception. An exemption rule for it was drafted and cut, since adding a value to the model's list of rules breaks `check.health`'s match on it for one use that is not yet needed (`yagni`).

Rejected:
- **A commit trailer naming each file outside the component**, as `path-ownership-by-prompt` rejected it.
- **Judging hunks, not files**, so that only the lines of a test citing the component may change in a shared file. A new test's imports and helpers lie outside any test, and the rule it would refine is transitional.
