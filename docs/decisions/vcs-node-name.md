---
title: "The plumbing node is `vcs`; its crate stays `clm-git`"
date: 2026-10-27
status: accepted
---

`git-plumbing-layer` named the plumbing node `git`, after its crate `clm-git`. Claude Code's worktree-isolation guard refuses a command that names git more than once, counting `git` alone as a word or path segment, so a build of that node stopped at its first command, `git switch impl/git`, and so did any `git diff -- specs/git`. The node is now `vcs`. Everything else `git-plumbing-layer` decided stands.

The crate keeps its name. `crates/clm-git` never tripped the guard (`clm-git` is one word to it), and the name says what the crate is: the git implementation of the node's job, beside the forge adapters in `clm-host` (GitHub, Azure DevOps), which are hosts of git repositories and not version control systems of their own. Were CLM ever to support another VCS, the node `vcs` would be the abstraction and a crate such as `clm-jj` would sit beside `clm-git`. Requirements that name the layer's API name the crate (`` `clm-git`'s `run` ``), not the node, so a later rename of the node changes no requirement's text.

Rejected:
- **Rename the crate to `clm-vcs` too.** Every dependant crate's `Cargo.toml` and imports would change in its next build, for a name the guard doesn't care about, and the crate's content is git-specific (`extract_tree`, `ls-files`, `merge-file`).
- **Add a check rejecting node IDs the guard would refuse.** The guard's rule is undocumented and word-based, `git` is the only node it catches, and a node it would catch fails loudly at its first command.
- **Start builds already on `impl/<node>`.** The build's later `git diff` and `git add` on `specs/git` would still be refused.
