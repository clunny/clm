---
title: "The release's prepare step leaves the Codex plugin's version behind"
kind: bug
components: [ci]
---

Step: building `plugin/codex-package`, which adds `plugin/.codex-plugin/plugin.json` with a `version` equal to the cargo workspace version, and a test that fails when they differ.

What happened: `.github/scripts/release-prepare.sh` bumps the workspace version, `plugin/.claude-plugin/plugin.json` and the panel's `plugin.json`, and nothing else. The next release bump leaves `plugin/.codex-plugin/plugin.json` at the old version, so the release's `cargo test --all` fails on `plugin`'s `codex_package` test and Codex's plugin update never sees the new version.

Expected: the prepare step sets the version in `plugin/.codex-plugin/plugin.json` beside the other two manifests and stages it with them.
