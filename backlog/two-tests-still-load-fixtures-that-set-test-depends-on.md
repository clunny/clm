---
title: "Two tests still load fixtures that set test_depends_on, so refusing the key would break them"
kind: question
components: [model, check.affected, context]
blocks: [model/test-dependencies-retired]
---

## Intent

`model/test-dependencies-retired` makes `test_depends_on` a key the model refuses. Two tests of other components load a component whose frontmatter sets it and assert the load reports nothing, so refusing the key fails them:

- `crates/clm-affected/tests/select.rs`, `a_component_whose_tests_name_a_changed_one_is_not_affected` (cites `check.affected/affected-components`)
- `crates/clm-context/tests/bundle_nodes.rs`, `test_dependency_is_not_in_the_bundle` (cites `context/bundle-contents`)

Those files belong to `check.affected` and `context`, whose requirements are current, so neither build has anything pending that would move the fixtures, and `model`'s build may not edit them (`path-ownership`). Nothing else reads the field: `Frontmatter::test_depends_on` is still declared, marked `clm:remove-by 20261113-temporary-exemptions`, and no code but those fixtures sets it.

## Open questions

- Should the two tests be dropped, since the key they check is no longer meant to exist, or rewritten to assert the refusal? Either needs a requirement of `check.affected` and of `context` to move them (`coupled-changes-sequence`), after which `model` removes the field, the accessor and the key from `KNOWN_KEYS`.
