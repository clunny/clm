---
title: "A human cuts a release by dispatching one workflow that bumps, builds, publishes, then updates main"
date: 2026-10-22
status: accepted
---

Answers `release-process`. The plugin's version is also the `v<version>` release its shim downloads `clm` from (`binary-from-releases`), and installed plugins only update when that version changes. So the version must change exactly when a release is published, never on its own in a PR. If parallel PRs bumped it they would conflict, and a bumped `main` with no release would break every shim that updated to it.

- **Trigger:** `workflow_dispatch` of `.github/workflows/release.yml` from `main`, with a `version` input. The human runs it from the Actions tab or with `gh workflow run release.yml -f version=X.Y.Z`. Releasing the current version without a bump is allowed, which is how the first release is cut.
- **Order:** bump commit on `release/v<version>` → tests → build every target from that commit → `gh release create` with every asset at once → merge the bump into `main`. `main` names a version only once its release exists, and a published release always has all its binaries. A failed run leaves nothing behind and can be rerun.
- **One workflow:** events made with `GITHUB_TOKEN` (a release, a push, a PR) start no workflows, so the steps can't be chained across workflows, and a bump PR opened by the workflow would get no CI. One dispatched workflow, with its own `cargo test`, does it all.
- **The bump skips the PR flow.** It's a mechanical edit of three version fields, made and tested by the workflow on a human's dispatch. That's no code authored outside the implement flow (`ai-writes-code`). Nothing else changes the version, so merging it into `main` can't conflict.
- **`humans-cut-releases`:** only a human releases. GitHub lets anyone with write access dispatch, and Claude shares the human's account, so the plugin's guard blocks `gh workflow run` of the release workflow, `gh release` writes, and `gh api` writes to dispatches or releases. Like the `spec-approved` guard, it's a backstop for accidents. It allows `gh run rerun`: the guard can't tell a release run's ID from a CI run's, Claude needs reruns for flaky CI, and a rerun only retries a release that a human dispatched with a version they chose. Naming the workflow by numeric ID also gets past the guard, which counts as obfuscation.
- **Recovery:** a run that fails before `publish` leaves nothing behind and is dispatched again. If the final push to `main` fails, the release is out and the job prints the commands that merge `release/v<version>`, for a human to run. A failed `web` deploy is rerun on its own.
- **Updating:** users run `claude plugin update clm` (or rely on marketplace auto-update). The new plugin's shim fetches the new binary on first use. CLM's own developers track `main` with `CLM_BIN`.
- **Release notes** come from `--generate-notes`, i.e. merged PR titles.

Rejected:
- Bumping in a normal PR, then publishing by hand. It leaves a window, or a forgotten step, where `main` names a version with no release.
- Publishing a release by hand in the GitHub UI to trigger the build. The assets arrive minutes after the release exists, and the version still has to be bumped separately.
- A bump PR opened and auto-merged by the workflow. Its CI would never run under `GITHUB_TOKEN` without a PAT or App token.
- Tag pushes as the trigger. Claude can push tags, and the guard can't tell a release tag from any other push.
