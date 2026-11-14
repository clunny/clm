---
title: "A rule's build updates the capability tests whose observations the rule changes"
date: 2026-11-05
status: accepted
---

A requirement of a product component can change what a capability test observes at the public boundary: `web/home-screen` changed the opening screen that `capabilities/web/`'s specs look at. Nothing gave that work a route. The `capability` component holds no requirements of its own (`human-layer-proof`), so `dependency-work-is-a-requirement` had no requirement to add and a `#### Needs` nothing to name; the queue builds the capability after the rule (`serving-rules-order-capabilities`), but the rule's own land runs the whole suite, old capability tests included, which then fail; and `path-ownership` forbade the rule's build from touching them. A project's draft improvised a `deferred` item fixed through `clm claim capability --fix`, which no prompt describes and which leaves `main` red between the two lands, if the first can land at all.

**Now.** The build of the requirement that changes what a capability test observes updates that test in the same branch. This is `path-ownership`'s one permanent exception for tests, because a capability test and the behaviour it observes can't land apart; any other change another component must follow is built in steps, each by its owner's build (`coupled-changes-sequence`). The fix changes only what the test expects to see, and the steps that reach it, where the requirement changed them (a new opening screen, a moved path, a reworded message), to what the requirement now states; it keeps the test's `req:capability/<slug>@<version>` citation, and drops no check the capability entry's scenarios state. The build names each one under `## Fixes in other components` in its pull request body, and the build's code review and spec audit read it there (`path-ownership-by-prompt`). The rule is the build's, written in `clm run`'s prompt, so it holds whether or not the change named the tests.

At design time the builder check of `/clm:propose` and of `clm draft` asks which capability tests each requirement changes what they observe, and the change names them in that requirement's text, as `web/app-at-app-path` already did, so the reviewer sees the move with the intent. It is neither a requirement of `capability` nor a `#### Needs`, and never a backlog item, which a `#### Needs` can't name (`requirement-needs`).

What stays outside the exception: a requirement under which a `CAPABILITIES.md` entry, as written, no longer holds. That is a change to the capability itself, so the change edits the entry (a hand edit, reviewed with it). The checks the fix keeps are those the entry states now, so the rule's build moves the tests to the entry's new text, leaving their citations at the old version; the capability is then `behind`, never counted as proven by tests of a text it no longer has (`proven-by-tests`), until its own build re-cites them after the rule (`serving-rules-order-capabilities`). A build that finds an entry the change missed, still unedited, can't keep its checks and pass: it leaves the requirement unbuilt and files a `question` item with `blocks` naming it (`requirement-blocks`).

A fix under this exception is not out-of-scope work under `fold-small-defer-large`: it is named under `## Fixes in other components`, so the change doesn't grow beyond its components unseen.

The tests are named at design time because the fix changes what the product visibly does, which the reviewer should see with the intent.

Why not the alternatives:
- **A requirement of `capability` for the move, named in a `#### Needs`.** The move and the rule can't land apart: the moved test fails until the rule is built, and the rule's land fails until the test moves, so the two `#### Needs` would form a cycle. And `capability` holds no requirements of its own by design.
- **A `deferred` item fixed with `clm claim capability --fix`.** The same split across two lands, with `main` red between them, and a `#### Needs` can't wait on an item.
- **Leave it to each change's requirement text, as `web/app-at-app-path` did.** That works only when the draft saw it; a change whose builder check missed a test leaves its build stuck. The build sees the failing test, so the rule belongs with the build, and the design-time naming is for the reviewer.

Refines `path-ownership`, `dependency-work-is-a-requirement` and `human-layer-proof`.
