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
