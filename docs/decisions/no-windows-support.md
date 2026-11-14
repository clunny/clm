---
title: "CLM supports Linux and macOS only; Windows support is cut"
date: 2026-11-04
status: accepted
---

An earlier design added a Windows release binary that the `sh` shim ran under Git Bash, and the path handling it needed in the shim and the panel. None of it ever ran on a Windows machine: the check on a real machine (below) was put off until CLM went public (`backlog/windows-check-after-going-public`), with the answer "or Windows support is cut if keeping it proves costly". Asked in triage on 2026-11-04, the human chose to cut it (`20261104-windows-check-after-going`).

The cut is the cheaper of the two. Keeping the support means a release job on `windows-latest`, a shim branch and a panel branch that only fakes test, and a check on a machine nobody here has, for users nobody has asked for; every later change to the shim, the panel or the release has to keep the untested paths in step (`yagni`).

- The release builds four targets: `x86_64-unknown-linux-musl`, `aarch64-unknown-linux-musl`, `x86_64-apple-darwin` and `aarch64-apple-darwin` (`ci/release`).
- The shim maps no Windows platform: under Git Bash or an MSYS2 shell it fails as on any platform without a prebuilt binary, naming none (`plugin/shim-targets`). It no longer has a rule about MSYS path conversion (`plugin/shim-path-conversion` removed).
- The panel reads every path as POSIX and runs the shim directly (`plugin.panel/panel-windows` removed).
- `crates/clm-init` no longer has to build for Windows (`init/windows-build` removed), so its Unix-only file-mode code needs no `#[cfg(unix)]` counterpart. Code elsewhere that sits under `#[cfg(unix)]` for its own reasons (`check`'s detector process groups, for one) keeps its fallback; no rule asks for a Windows build of it.
- Windows Subsystem for Linux is Linux: the shim there downloads the Linux binary, as it always did.

Rejected: running the Windows check first (needs a Windows machine with Git for Windows and Claude Code, which the human chose not to spend); keeping the release asset but dropping the shim and panel paths (an asset no shim downloads is dead weight on every release).

## What adding Windows back would start from

Adding Windows back is a change of its own. It would start from the design that was cut, with the check at the end of this section run on a real machine first.

**The binary.** Claude Code on native Windows runs the Bash tool, and plugin hooks written as shell commands, through Git Bash when Git for Windows is installed (Claude Code's setup and hooks documentation, read 2026-10-27). The plugin's `bin/clm` is a POSIX `sh` script, so under Git Bash it runs as it does elsewhere, given a Windows binary to download.

- The release built one more target, `x86_64-pc-windows-msvc`, on `windows-latest` with the MSVC tools the image ships. One target only: Windows on Arm runs x86_64 binaries under emulation, so the shim sent `aarch64` Git Bash to the x86_64 asset too; an `aarch64-pc-windows-msvc` build would need a cross C toolchain for the tree-sitter grammars.
- The asset was `clm-x86_64-pc-windows-msvc.tar.gz` holding `clm.exe`, not a `.zip`: Git Bash's `tar` reads it, so the shim kept one unpacking path and the pack step was the same script on every target.
- The shim recognised Git Bash by `uname -s` starting with `MINGW` or `MSYS`, and MSYS2's UCRT64, CLANG64 and CLANGARM64 shells, whose `uname -s` names their environment (`UCRT64_NT-…`, `CLANG64_NT-…`, `CLANGARM64_NT-…`) and which run the same native `.exe`. Those were matched by the three prefixes, not by `*_NT-*`, which would also take Cygwin's `CYGWIN_NT-…`; Cygwin wasn't mapped, since Claude Code doesn't use it.
- Without Git Bash, Claude Code falls back to PowerShell, where a `sh` script doesn't run. That case stayed unsupported; Claude Code's own troubleshooting tells the user to install Git for Windows.
- `clm-init` sets the check script's mode with `std::os::unix`, which doesn't build for Windows. It did so only on Unix and, elsewhere, told the human to record the mode with `git update-index --chmod=+x`.
- Rejected then: a PowerShell or `clm.cmd` shim beside the `sh` one (two shims to keep in step, for a setup Claude Code itself discourages); `x86_64-pc-windows-gnu` (needs MinGW on the runner, while MSVC is already there).

**Paths.** A POSIX path meets a native Windows process in two places: MSYS's conversion of arguments and environment values when Git Bash execs `clm.exe`, and the panel, a hooks module in Node, starting the `sh` shim.

- The shim left MSYS's argument conversion on. Git Bash rewrites an argument that looks like an absolute POSIX path (`/tmp/body.md`, `/c/Users/u/x`) into the Windows path it names (`C:/Users/u/AppData/Local/Temp/body.md`, `C:/Users/u/x`) when it execs a native program. `clm` takes such paths (`--body-file`, a scratchpad file), and `clm.exe` can open only the converted form, so `MSYS2_ARG_CONV_EXCL='*'` or `MSYS_NO_PATHCONV=1` would break them. The cost is that a text argument starting with `/`, such as a title `/clm:view fails`, may be rewritten too; skills and prompts pass long text through files, and the rest was accepted until a Windows user reported it.
- The shim exports no path of its own to convert, so nothing needed `cygpath -m`.
- The panel ran the shim through Git Bash's `bash.exe`, found from `git --exec-path`. Node's spawn can't start an extensionless `sh` script on Windows. A bare `bash` on PATH is often absent (Git for Windows puts only `Git\cmd` on PATH by default) and, where WSL is installed, is `C:\Windows\System32\bash.exe`, which would run the shim in Linux. `git` is on PATH wherever Claude Code's Bash tool works, and `git --exec-path` prints `<Git>/<mingw64|clangarm64>/libexec/git-core`, three levels below the Git root whose `bin/bash.exe` Claude Code itself uses. The panel recognised Windows by a drive-letter `$.plugin.root`, since a hooks module has no platform call, and normalised `\` to `/`, which Windows accepts, so its path rules stayed one set of string functions.
- Rejected then: having the panel run the cached `clm.exe` itself (it would duplicate the shim's version and cache rules, and can't read `XDG_CACHE_HOME` or `HOME`); `MSYS2_ARG_CONV_EXCL='*'` (breaks every absolute path argument to protect rare text ones).

**The check, never run.** On a Windows machine with Git for Windows and Claude Code, in a CLM repository, have the Bash tool run `clm status` and `clm pr open ... --body-file <an absolute path>`, and run `/clm:view` with `clm-panel` installed. Report what fails with `clm defect`, for instance `git --exec-path` printing a form the panel doesn't expect, `$.plugin.root` not starting with a drive letter, or the shim, started by the panel through `Git/bin/bash.exe`, missing `uname`, `tar`, `sha256sum` or `gh` on its `PATH`, or `HOME`.
