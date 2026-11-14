---
title: "Two tests still load fixtures that set test_depends_on, so refusing the key would break them"
kind: deferred
components: [model, check.affected, context]
blocks: [model/test-dependencies-retired]
---

## Intent

`model/test-dependencies-retired` makes `test_depends_on` a key the model refuses. Two tests of other components load a component whose frontmatter sets it and assert the load reports nothing, so refusing the key fails them:

- `crates/clm-affected/tests/select.rs`, `a_component_whose_tests_name_a_changed_one_is_not_affected` (cites `check.affected/affected-components`)
- `crates/clm-context/tests/bundle_nodes.rs`, `test_dependency_is_not_in_the_bundle` (cites `context/bundle-contents`)

Those files belong to `check.affected` and `context`, whose requirements are current, so neither build has anything pending that would move the fixtures, and `model`'s build may not edit them (`path-ownership`). Nothing else reads the field: `Frontmatter::test_depends_on` is still declared, marked `clm:remove-by 20261113-temporary-exemptions`, and no code but those fixtures sets it.

## Answer (2026-11-13, by the human)

Drop both tests. Neither `check.affected/affected-components` nor `context/bundle-contents` states anything about test dependencies any more, so the two tests prove nothing their requirements state; once the key is refused the situation they set up can't exist; and rewriting them to assert the refusal would put `model`'s proof in two other components' files, when `model/test-dependencies-retired` has its own scenario for it.

## What is left to do

Delete the two test functions named above, and nothing else in those files. No requirement or scenario changes: other tests cite both requirements (`crates/clm-affected/tests/select.rs` and `crates/clm/tests/affected.rs` for the first, `crates/clm-context/tests/served.rs` and `crates/clm/tests/context.rs` among others for the second), so both stay current. The work is code only, on a branch whose commit deleting this item holds the trailer `Resolves: backlog/two-tests-still-load-fixtures-that-set-test-depends-on` (`deferred-work-resolves-in-code`).

This item is `deferred`, since nothing waits on the human any more; it keeps `model/test-dependencies-retired` blocked until the tests are gone. That requirement also needs `check.affected/why-chain` and `check/undeclared-imports`, which are still pending, so deleting this item alone doesn't make it ready. `model`'s build then removes the field, the accessor and the key from `KNOWN_KEYS`.
