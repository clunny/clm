---
title: "`clm init` writes in the CLM repository it finds; `clm survey` maps the whole git repository before init"
date: 2026-10-23
status: accepted
---

`root-from-cwd` makes a command with no CLM repository use the current directory. Two commands don't follow it:

- **`clm survey`**, with no CLM repository, maps the top of the git repository. Before `clm init` there's never a CLM repository, and a brownfield adoption maps the whole codebase (`brownfield-survey`), so `/clm:init` gets the same map from any directory. Inside a CLM repository, or with `--root`, it follows `root-from-cwd`.
- **`clm init`**, which takes no `--root`, writes in the CLM repository it finds as `root-from-cwd` finds it, and at the top of the git repository only when there is none. Rerunning it after an upgrade, from a subdirectory or in a CLM repository that is a subdirectory such as `tool/`, regenerates that repository's files instead of starting a second CLM repository at the top.

Every file `clm init` writes goes in that repository, `.github/` included. GitHub runs workflows only from the top of the git repository, so a CLM repository in a subdirectory gets no CI from `clm init`. Only the CLI works there today, and CI for it is deferred (`backlog/ci-for-subdirectory-clm-repositories`). There, `clm init`'s next steps say so instead of asking for Actions, auto-merge and required checks, because a required check that never reports would block every merge.

`clm survey`'s exception is stated in `root-from-cwd`, beside the rule, and `clm-init` repeats the search for `clm init`, because it doesn't depend on `clm-hooks`, which holds `clm_root`.

Rejected:
- **`clm survey` follows `root-from-cwd` too**, mapping only the current directory's subtree, with `/clm:init` running it from the top. It's more uniform, but a survey started from a subdirectory would quietly miss most of the codebase.
- **Writing `.github/` at the git top level** for a subdirectory CLM repository, with the workflow working in `tool/`. It's more to design and test, for a layout nothing uses yet.
