---
title: "The gate refuses a build's change to a file that isn't its to change, once the root names its wiring"
date: 2026-11-14
status: accepted
---

`path-ownership` was kept by the build prompt alone, and a sentence in a prompt enforces nothing (`three-kinds-of-rule`). A gate rule had been weighed on 2026-10-30 and set aside for its edge cases. Most are gone: builds no longer make compile-only fixes in other components, and the commits `clm rebase` adds touch only spec paths.

For a branch named `impl/<component>`, each changed code file is refused that is none of: the component's own; a file no component owns; a file the root's `wiring` globs name; a file that cites the component (`req:<component>/` at the merge base or at the head); a capability test, edited and not added or deleted, citing a capability the component serves. The gate reads no branch name (`spec-branches-hold-spec-paths`), so it offers the list as a library call and `clm land`, which knows the branch, refuses; `clm land --check-only` says so first.

**The switch.** The rule is in force once the root sets `wiring`, even to `[]`, as `check`'s path ownership is in force once it sets `outside`. A project that was green before it upgraded stays green, and adopts the rule by naming its wiring.

**Why "a file that cites the component".** Of the last 429 builds here, 180 edited a test file another component owns that cites the building component: Cargo gives the binary only to the root package's tests, so 36 files under `crates/clm/tests/` prove other components, and 18 more sit in a parent's or a sibling's crate. The gate already reads where a test belongs this way for a code fix (`fixes-count-where-cited`). The alternative was an exemption per owner: eleven entries, each opening its files to every build and each needing review. The clause opens a file only to the builds it cites, covers adding and deleting as well as editing, which settles `backlog/the-written-exception-to-path-ownership-lets-a-build-re-cite-a`, and goes when each crate proves its requirements through its own entry point (`backlog/clm-s-cli-tests-prove-other-components-through-the-binary`).

**What it leaves to judgement.** That a wiring edit adds only the build's own entry, and that a capability-test edit keeps what the capability's scenarios state. A branch named outside `impl/` has no component and is not judged, so the small-change route is not held to it.

With that clause and this repository's wiring, 29 of those 429 builds (7%) would have been refused: most changed a root test file that cites another component, as when a change to the ledger's records broke `land`'s CLI tests, and the rest a parent's shared test helper, a child's source file or another component's code. Those are the coupled changes `expand-contract` builds in steps. One more build was `init` writing `upgrade`'s migrations alone, the written exception `template-change-ships-its-migration`. The rule has no clause for it. An exemption rule for it was drafted and cut, since adding a value to the model's list of rules breaks `check.health`'s match on it for one use that is not yet needed (`yagni`). This decision first held that this repository would set `wiring` only once a template and its migration shared a component or the rule had a way to keep the exception; the human chose on 2026-11-14 to set it first and let such a build wait, as the last section says.

Rejected:
- **A commit trailer naming each file outside the component**: a convention invented to work around the rule on its first day.
- **Judging hunks, not files**, so that only the lines of a test citing the component may change in a shared file. A new test's imports and helpers lie outside any test, and the rule it would refine is transitional.

**Switched on here (2026-11-14, `20261114-proof-switched-on`).** This repository's root names its wiring: `Cargo.toml`, `Cargo.lock`, `crates/clm/Cargo.toml`, `crates/clm/src/main.rs` and `crates/clm/src/upgrade_gate.rs`. `ARCHITECTURE.md`'s `path-ownership` is a checked entry from that land, in force for a project once its root names its wiring.

The list is what builds touched. The real `foreign_files` was run over the last 300 `impl/*` lands, back to 2026-11-08, with a wiring list given to it. With an empty list 48 would have been refused. The 27 a list saves changed, outside their own files, only `crates/clm/src/main.rs` (15 edits), `Cargo.lock` (14), `crates/clm/Cargo.toml` (4), `crates/clm/src/upgrade_gate.rs` (4), `clm.toml` (4) and `Cargo.toml` (2). The earlier draft's `crates/clm/src/**` opened four more source files of the root that no other build touched. `clm.toml` is left out: it is `config`'s, and each of this repository's settings is a requirement of `config` that its build makes (`config/verify-here`), so the four builds that edited it would rightly have waited for `config`'s.

With that list 21 of the 300 (7%) would have been refused:
- ten were a split's child build that also edited or deleted the parent's copy, which `nested-splits` gives to the parent's build: nine builds of the `plugin.*` skill components, since removed, in `crates/clm/tests/plugin.rs`, and `web.views` in 39 of `web`'s files;
- four were `init` writing `upgrade`'s migration and mending `upgrade`'s tests;
- seven were coupled changes: a fix of 17 files in ten components on one branch, `check` editing `check.affected`'s source, `ledger` editing `land`'s, and four edits to another component's tests or to the capability suite's harness.

**No clause for a template and its migration.** Each of the four `init` builds also edited `upgrade`'s existing tests, so naming the migration files in the wiring would have let none of them through, and it would have opened `upgrade`'s files to every build for good. An exemption can only cover a component's own files, so one for this would sit on `upgrade` and be the same thing with an end date. The exception exists because one behaviour, what a project set up earlier holds, is split over two components. So the rule refuses it, and a change of its own puts the migrations of the files `clm init` writes with those files (`backlog/a-template-and-its-migration-live-in-different-components`). Until it is built, a requirement that changes such a file waits behind a question item.

**No clause for a fix that spans components.** `clm claim <component> --fix` let one branch carry a fix across every component a `bug` or `deferred` item names. The rule refuses the files of the other components, and no clause was added for them: work is per component (`component-reconciliation`), so each component's part is fixed under its own claim and the last deletes the item.
