---
title: "path-ownership is kept by the build prompt; no gate or land check enforces it"
date: 2026-10-30
status: accepted
---

`path-ownership` says a component's build changes only its own component's files: those its `tests` match (`components-own-their-tests`) and those its `paths` match that no other component's `tests` match. A change another component must follow is built in steps, each in its own component's files, so no build makes a fix in another's code (`coupled-changes-sequence`). The exceptions are few, and the rule in full, below, names them: the shared wiring, a capability test whose observations a build's requirement changes (`capability-tests-follow-rules`), and two transitional ones, the `upgrade` migration a template change ships (`template-change-ships-its-migration`) and a stale marker of the build's own requirement in a file no component's `tests` match. Only `clm run`'s prompt says all this. A gate rule refusing an `impl/<component>` branch's files outside the component's own was considered and is not built: the gap is accepted (`redesign-before-workaround`).

Why not a gate rule. The gate already maps a file to a component (`code-fix-resolves-bug`'s `:(glob)` match) and reads commit trailers, so the check itself would be small. What isn't small is its list of edge cases:
- the exceptions above, which the gate can't read from the page, so each would need a commit trailer naming its files: a convention invented to work around the rule on its first day;
- tests kept under another component's `paths` because of Cargo (`plugin`'s skill tests in the root's `crates/clm/tests/plugin.rs`, `fixes-count-where-cited`), which a path check would refuse, until the project gives its tests to their components;
- files every build may touch and no component owns alone: the root's dispatch files, `Cargo.lock` and the workspace manifests;
- the commits `clm rebase` adds to an `impl/*` branch (the `Activate` edit, archives, regenerated `specs/`) and backlog items a build files.

When the rule was first weighed the list was longer, since builds then also made the compile-only fixes a removal or an addition forced in other components' code, and that many exceptions on one rule is the red flag the principle names. With so few left, a gate rule is worth weighing again. Meanwhile the harm the rule prevents is bounded: `clm land` runs the checks on the merged tree, so a stray edit that breaks another component fails there; and the build's code review and spec audit read the whole diff, so an edit outside the component's files is visible before it lands.

Revisit when a build is seen changing another component's code outside the exceptions and a review missed it; file that as a defect, and weigh the rule against its edge cases then.

Rejected:
- **A gate rule with a `Forced-by:` trailer per file.** The edge cases above.

## The rule in full

`ARCHITECTURE.md`'s entry `path-ownership` states the rule in short, without its exceptions (`human-layer-entries-stay-short`); the rule in full:

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
