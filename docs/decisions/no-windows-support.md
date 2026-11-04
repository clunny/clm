---
title: "CLM supports Linux and macOS only; Windows support is cut"
date: 2026-11-04
status: accepted
---

`windows-binaries` added a Windows release binary that the `sh` shim runs under Git Bash, and `windows-paths-in-shim-and-panel` added the path handling it needs in the shim and the panel. None of it ever ran on a Windows machine: the check that decision describes was put off until CLM went public (`backlog/windows-check-after-going-public`), with the answer "or Windows support is cut if keeping it proves costly". Asked in triage on 2026-11-04, the human chose to cut it (`20261104-windows-check-after-going`).

The cut is the cheaper of the two. Keeping the support means a release job on `windows-latest`, a shim branch and a panel branch that only fakes test, and a check on a machine nobody here has, for users nobody has asked for; every later change to the shim, the panel or the release has to keep the untested paths in step (`yagni`).

- The release builds four targets: `x86_64-unknown-linux-musl`, `aarch64-unknown-linux-musl`, `x86_64-apple-darwin` and `aarch64-apple-darwin` (`ci/release`).
- The shim maps no Windows platform: under Git Bash or an MSYS2 shell it fails as on any platform without a prebuilt binary, naming none (`plugin/shim-targets`). It no longer has a rule about MSYS path conversion (`plugin/shim-path-conversion` removed).
- The panel reads every path as POSIX and runs the shim directly (`plugin.panel/panel-windows` removed).
- `crates/clm-init` no longer has to build for Windows (`init/windows-build` removed), so its Unix-only file-mode code needs no `#[cfg(unix)]` counterpart. Code elsewhere that sits under `#[cfg(unix)]` for its own reasons (the runner's process groups, `check`'s detector process groups) keeps its fallback; no rule asks for a Windows build of it.
- Windows Subsystem for Linux is Linux: the shim there downloads the Linux binary, as it always did.

Supersedes `windows-binaries` and `windows-paths-in-shim-and-panel`. Adding Windows back is a change of its own, starting from those two decisions and the check the second one describes, run on a real machine first.

Rejected: running the Windows check first (needs a Windows machine with Git for Windows and Claude Code, which the human chose not to spend); keeping the release asset but dropping the shim and panel paths (an asset no shim downloads is dead weight on every release).
