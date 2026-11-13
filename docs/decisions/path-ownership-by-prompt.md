---
title: "path-ownership is kept by the build prompt; no gate or land check enforces it"
date: 2026-10-30
status: accepted
---

`path-ownership` says a node's build changes only its own `paths`, except for the compile-only and test-fixture fixes a removal forces in other nodes' code, which the page names. Only `clm run`'s prompt says so (`orchestrator.run/removal-breaks-dependant`). A gate rule refusing an `impl/<node>` branch's files outside the node's `paths` was considered and is not built: the gap is accepted (`redesign-before-workaround`, under the re-founding's stop on growth).

Why not a gate rule. The gate already maps a file to a node (`code-fix-resolves-bug`'s `:(glob)` match) and reads commit trailers, so the check itself would be small. What isn't small is its list of edge cases, all known before building it:
- the removal exception, which the gate can't read from the page, so it needs a new commit trailer naming each file: a convention invented to work around the rule on its first day;
- tests kept under another node's `paths` because of Cargo (`plugin`'s skill tests in the root's `crates/clm/tests/plugin.rs`, `fixes-count-where-cited`), which a path check would refuse;
- files every build may touch and no node owns alone, such as `Cargo.lock` and workspace manifests;
- the commits `clm sync` adds to an `impl/*` branch (the `Activate` edit, archives, regenerated `specs/`) and backlog items a build files.

That many exceptions on one rule is the red flag the principle names. Meanwhile the harm the rule prevents is bounded: `clm land` runs the checks on the merged tree, so a stray edit that breaks another node fails there; and the build's code review and spec audit read the whole diff, so an edit outside the node's `paths` is visible before it lands.

Revisit when a build is seen changing another node's code outside the exception and a review missed it; file that as a defect, and weigh the rule against its edge cases then.

Rejected:
- **A gate rule with a `Forced-by:` trailer per file.** The edge cases above.
- **Expand-contract for every removal, so no build ever touches another node.** Turns each removal into several builds across nodes; the re-founding's touchpoint 2 chose the named-fix exception instead.

(Amended 2026-11-04, `20261104-clm-run-allows-compile`: the exception covers the fixes an addition forces as well as a removal (`forced-fixes-cover-additions`); the edge cases above are unchanged.)

(Amended 2026-11-05, `20261105-capability-harness-move-has`: the exception also covers a build updating a capability test whose observations its requirement changes (`capability-tests-follow-rules`); the edge cases above are unchanged.)

(Amended 2026-11-09, `20261109-land-still-sets-cargo`: the exception also covers a build that changes a file `clm init` writes writing the `upgrade` migration that carries it, as stated (`template-change-ships-its-migration`); the prompt says so (`orchestrator.run/template-migration-fix`), and the edge cases above are unchanged.)

(Amended 2026-11-12, `20261112-path-ownership-names-no`: the exception also covers a build whose requirement a change moved to its component from another, as a split does (`nested-splits`), moving the code and the tests that can move out of the other component's `paths`, rewiring that component's imports to reach the moved code and re-pointing the moved requirement's markers; the prompt says so (`orchestrator.run/split-move-fix`), and the edge cases above are unchanged, the tests kept under another node's `paths` because of Cargo among them.)

(Amended 2026-11-12, `20261112-components-own-their-tests`: the forced-fix and split-move exceptions are gone (`coupled-changes-sequence`), a component's tests are its own through `tests` globs (`components-own-their-tests`), and what remains is one permanent exception, capability tests, and two transitional ones. With so few edge cases left, a gate rule is worth weighing again; the root's dispatch files and the commits `clm rebase` adds are still among them.)

## The rule in full

`ARCHITECTURE.md` stated this as the entry `path-ownership` until 2026-11-13, when the entry was cut to its rule and the exceptions moved here (`human-layer-entries-stay-short`). The text still holds:

A component's build SHALL change only its own component's files: those its `tests` match, wherever they sit, and those its `paths` match that no other component's `tests` match (`components-own-their-tests`). A change that another component's code or tests must follow is built in steps, each by the build of the component whose files it changes: the provider expands, each user moves in a requirement of its own that needs the expansion, and the provider contracts once every move is built (`coupled-changes-sequence`, `expand-contract`); a build whose requirement needs an edit outside its files leaves it unbuilt with a `question` item. Two exceptions are permanent. A build edits the shared wiring its root's Design names, the files that wire every component into one product (here the root's dispatch files, `crates/clm/src/main.rs`, `Cargo.toml` and `Cargo.lock`, and `clm.toml`'s detector allowances, `unreferenced-allow-then-narrow`), only to add or change the entry its own component needs. And a build whose requirement changes what a capability test observes at the product's boundary updates that test's expectations, and the steps that reach them, to what the requirement states, keeping its citation and every check its capability's scenarios, as `CAPABILITIES.md` now states them, state, and names each one in its pull request; a requirement under which the entry as written no longer holds is left unbuilt, with a `question` item, until a change edits the entry, since its own land runs that test and the `capability` component holds no requirement to carry the move (`capability-tests-follow-rules`): a capability test observes the whole product, so a behaviour and the test that observes it can't land apart. Two are transitional, each until a backlog item is done: a build whose requirement changes the text of a file `clm init` writes writes the `upgrade` migration that carries the change, as the `upgrade` requirement that needs it states it, brings `upgrade`'s tests that the new migration makes fail to what `upgrade`'s requirements as applied state, and names both in its pull request, since the upgrade corpus test fails a land of either half alone (`template-change-ships-its-migration`), until a template and its migration share a component (`backlog/a-template-and-its-migration-live-in-different-components`); and a build may re-cite, and bring up to the current text, a test in a file no component's `tests` match that cites one of its own requirements at another version, since that test is part of the cited component's build (`tests-count-where-cited`, `stale-markers-keep-behind`), until the project gives its tests to their components (`backlog/clm-s-cli-tests-prove-other-components-through-the-binary`).

**Scenario: test in its own directory of a shared suite**
- GIVEN a suite directory `tests/` that a harness component's `paths` own, and component `a` with `tests: ["tests/a/**"]`
- WHEN `a`'s build adds a test
- THEN it writes it under `tests/a/`, reaching `a`'s code through `a`'s own surface and what `a` depends on, and changes no file of the harness

**Scenario: removal another component still uses**
- GIVEN a build of `a` whose requirement removes a Contract function that component `b`'s code still calls, and no requirement of `b` that moves it off
- WHEN it builds
- THEN it leaves the requirement unbuilt, keeps `b` building, and adds a `question` item whose `blocks` names the requirement

**Scenario: split sequenced**
- GIVEN a change that moved requirement `a/x` to its new child `a.b`, and gave `a` a requirement that removes its copy and reaches the code through `a.b`, needing `a.b/x`
- WHEN `a.b` builds, then `a`
- THEN `a.b`'s build writes the code and its tests in `a.b`'s files, and `a`'s build deletes its copy and rewires its imports, each in its own component's files

**Scenario: rule changes what a capability test sees**
- GIVEN a build of `web` whose requirement changes the opening screen that a capability test under `capability`'s `paths` checks
- WHEN it builds
- THEN it updates that test to expect the new screen, keeping the capability it cites, and its pull request names the fix

**Scenario: template change carries its migration**
- GIVEN a build of `init` whose requirement changes the starter check script, and an `upgrade` requirement stating the migration that carries it and needing that requirement
- WHEN it builds
- THEN it writes that migration as stated, and its pull request names the fix

**Scenario: stale marker in another component's files**
- GIVEN a build of `a` whose requirement `a/x` is cited at an older version by a test in `b`'s `paths`
- WHEN it builds
- THEN it re-cites that test at `a/x`'s current version, fixing the test first where it no longer proves the current text

**Untested**
Accepted: no gate or land check refuses a build's file outside its component's files. The build prompt keeps it there, and the build's review and spec audit read the diff (`path-ownership-by-prompt`). Which files a component owns is checked: `paths` and `tests` stay disjoint, and a test a component's `tests` own cites only that component.
