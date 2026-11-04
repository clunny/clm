---
status: applied
resolves:
  - windows-check-after-going-public
---

# Cut Windows support

## Intent

CLM's Windows paths (a release binary built on `windows-latest`, the shim's Git Bash and MSYS2 mapping, the shim's MSYS path-conversion rule, the panel's drive-letter paths and Git Bash launch, and `clm-init`'s Windows build) have never run on Windows. `backlog/windows-check-after-going-public` asked the human to run the check `windows-paths-in-shim-and-panel` describes, or to cut Windows support if keeping it proves costly. The human chose the cut. This change drops the Windows paths from `plugin`, `plugin.panel`, `ci` and `init`, rewords the Design lines in `check`, `orchestrator.runner` and the root that justified code by the Windows release, records the cut as `no-windows-support` (superseding `windows-binaries` and `windows-paths-in-shim-and-panel`), and deletes the item.

## The human's answers

> Cut Windows support: draft a spec change that drops the Windows paths and deletes this item.

## Problem

The Windows support costs a release job on a hosted Windows runner, a branch in the shim and one in the panel that only fakes test, and a check on a Windows machine with Git for Windows and Claude Code that nobody has run. Every later change to the shim, the panel or the release has to keep those untested paths in step, for users who haven't asked for them (`yagni`).

## Approaches

- **Cut it everywhere it lives (chosen, the human's answer).** The shim maps four Unix targets and fails on Git Bash as on any platform without a binary; the release builds those four; the panel's Windows rule and `clm-init`'s Windows build rule go; the decisions that added them are superseded by one that says why.
- **Run the check first.** Rejected by the human: it needs a Windows machine and their time.
- **Drop the shim and panel paths but keep the release asset.** Rejected: an asset no shim downloads is dead weight on every release (`no-dead-code`).

## Outcome

- `plugin/shim-targets` maps exactly the four Linux and macOS targets; a `MINGW`/`UCRT64` `uname` gets the unsupported-platform failure, and the shim's text names no Windows target or `clm.exe`. The Windows and MSYS2 scenarios become one `Windows unsupported` scenario, and the Cygwin scenario, which only marked the edge of the Windows mapping, goes.
- `plugin/shim-path-conversion` is removed: with no Windows binary there is no MSYS conversion to keep on.
- `plugin.panel/panel-windows` is removed: the panel reads every path as POSIX and runs the shim directly.
- `ci/release` builds and publishes four targets, with no `windows-latest` job and no `clm.exe`, and no longer needs `init/windows-build`. Its Design loses the Windows bullet and the Git Bash `tar` rationale.
- `init/windows-build` is removed; `init`'s Design no longer cites it for `build.rs`'s `{:?}` paths, which stay as they are.
- `check`'s and `orchestrator.runner`'s Designs and the root's `closed-stdout` paragraph no longer justify their code by a Windows release; the code they describe stays.
- `docs/decisions/no-windows-support.md` records the cut; `windows-binaries` and `windows-paths-in-shim-and-panel` are marked superseded by it, and `binary-from-releases` says there is no Windows binary.

What the builds remove (`no-dead-code`): the shim's Windows `case` arms and `clm.exe` handling and their tests in `crates/clm/tests/plugin.rs`; the panel's drive-letter and `git --exec-path` code in `plugin-panel/src/panel.ts` and its tests; the `x86_64-pc-windows-msvc` matrix entry and Windows-only steps in `.github/workflows/release.yml`, and the release workflow test's Windows expectations; `crates/clm-init/tests/windows_build.rs` and `clm-init`'s non-Unix file-mode counterpart and its `git update-index --chmod=+x` next step, the Unix code losing its `#[cfg(unix)]` gate.

## Constraints

- Build order stays as `shim-assets-before-release` set it: `plugin/shim-targets` needs `ci/release`, so the matrix narrows first. Until the shim's build lands, `release-prepare.sh` refuses a dispatch, naming `x86_64-pc-windows-msvc`, because the shim still maps it; that refusal is the decision's accepted window, and nothing is published broken.
- A plugin version released before this keeps downloading only its own version's release, which holds its Windows asset, so existing installs are unaffected until they update.
- Windows Subsystem for Linux reports `Linux` and keeps getting the Linux binary.

## Edge cases and workarounds

None known.
