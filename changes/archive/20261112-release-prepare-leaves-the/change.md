---
status: done
resolves: [release-prepare-leaves-the-codex-plugin-version-behind]
---

## Intent

Make the release's prepare step set the `version` in `plugin/.codex-plugin/plugin.json` beside the other two plugin manifests, and stage it in the bump commit with them, so a release bump keeps the Codex package's version equal to the workspace version (`plugin/codex-package`) and Codex's plugin update sees the new version.

## Problem

`plugin/codex-package` added `plugin/.codex-plugin/plugin.json`, whose `version` must equal the cargo workspace version, with a test that fails when they differ. `ci/release` names the manifests the bump commit sets, `plugin/.claude-plugin/plugin.json` and `plugin-panel/.claude-plugin/plugin.json`, and `.github/scripts/release-prepare.sh` sets exactly those. The next bump therefore leaves the Codex manifest at the old version: the release's own `cargo test --all` fails on `plugin`'s `codex_package` test, so no release with a bump can be cut, and were it cut, Codex would never see the new version. No requirement states the Codex manifest's bump, so the fix is a spec change (`code-fix-needs-a-written-rule`).

## Approaches

1. **Name the Codex manifest in `ci/release` and its `higher version` scenario, and in the Design's bullet** (chosen). The script adds it to its manifest loop and its `git add`; its test fixture gains a Codex manifest. One line of rule, the smallest fix that keeps the rule and the script listing the same files.
2. **Have the script find every `plugin.json` under `plugin*/`.** Saves naming a future manifest, but a glob would also bump any manifest that shouldn't move with the workspace, and the rule would stop saying which files the bump commit holds (`yagni`).
3. **Drop the Codex manifest's `version`.** `plugin/codex-package` requires it, and Codex's plugin update reads it; rejected.

## Outcome

- `ci/release` lists `plugin/.codex-plugin/plugin.json` among the files the bump commit sets to `version`, and its `higher version` scenario checks it.
- The Design's bump bullet names the Codex manifest and `plugin/codex-package`.

## Constraints

- Building `ci/release` updates `.github/scripts/release-prepare.sh` (the manifest loop and the `git add` line) and the prepare-script test's fixture, which must hold a `plugin/.codex-plugin/plugin.json` for the `higher version` scenario.
- No edge cases or workarounds are known.
