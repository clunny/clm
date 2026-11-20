---
title: "No rule states that a root's refused `optional: true` leaves its tree required"
kind: question
components: [model]
choices:
  - "State it in optional-nodes"
  - "Let the root's flag count"
---

`model/optional-nodes` says a component is optional when it or any of its ancestors sets `optional: true`, and that `optional: true` on the root is refused. The code ignores the root's flag when it judges a descendant (`Graph::is_optional` skips the root, the comment above it says why), so a root that sets the flag still has `b` depending on an optional `a` reported. `crates/clm-model/tests/model.rs::optional_root_is_rejected` asserts that, but read literally the requirement makes every component optional once the root sets the flag, and then `b` would not be reported.

Found by the spec audit of `model`'s wave-1 build (`20261119-wave-1-by-the-rule`), which changes no behaviour, so the code and the test stay as they are. A spec change should state the rule (the root's refused flag makes nothing optional) or say otherwise, so the test's second half cites something that states it (`code-fix-needs-a-written-rule`).
