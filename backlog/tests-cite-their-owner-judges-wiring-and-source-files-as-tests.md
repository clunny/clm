---
title: "tests-cite-their-owner judges the wiring files and calls every owned file a test"
kind: question
components: [check]
---

Once the root sets `wiring`, `check/tests-cite-their-owner` (at `55d31d1a`) judges a marker in any file a component owns. Two consequences the code review of the build that made it current found, neither of which the requirement's text settles:

- The root's `wiring` files (here `crates/clm/src/main.rs`, `crates/clm/src/upgrade_gate.rs` and the manifests) are owned by the root, so a marker there citing another component is reported. Yet every build may edit those files (`gate/own-files`), so the requirement's stated reason, that the cited component's build could never re-cite the marker, doesn't hold for them. Should a file the `wiring` globs match be left unjudged?
- The message still reads ``the file is `a`'s test; move the test to `b`'s tests``, which misleads for a marker in a source file that `paths` alone own. Should the wording say "file" and offer a fix that fits a source file?

`clm check` reports neither on this repository today. A spec branch should state the answer in `check/tests-cite-their-owner`.
