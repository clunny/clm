---
component: view.repo
base: { design: "7d57453b7baf934f" }
---

## MODIFIED Design

**At a commit** (`view-at-rev`). `--at <rev>` swaps only where the files come from. The commit's blobs are read from the local objects in memory with `clm_git::tree_blobs` (`vcs/tree-blobs`) rather than extracted as `clm next` extracts a tree, which keeps the cost low enough for the pane's 30-second redraw. They go to the same in-memory calls the `web` Worker uses (`clm_model::load_files`, `clm_coverage::markers_in`, `clm_check::size_of`), so `--at` marks what `clm view spec` would at that commit. When the CLM root lies below the repository's top, only the paths under it are kept, made relative to it; `linguist_excluded` comes from the commit's `.gitattributes`, and a binary blob, as git judges it, gives no markers. The budgets come from that commit's `clm.toml` alone (`clm_config::node_size_from`): the view describes the commit, so `clm.local.toml` and the environment don't apply.

**`<age>`.** Git has no command that says when a remote was last fetched, and the reflog of `refs/remotes/origin/main` says when the ref last moved and doesn't apply to a `<rev>` that is no ref, so the `FETCH_HEAD` files are read directly. `FETCH_HEAD` is per worktree while `origin/*` is shared by every worktree, so every worktree's file counts (`vcs/common-dir`), including one removed without `git worktree prune`, since it recorded a real fetch of the shared refs, and a fetch of another remote or a `git pull`, since it writes `FETCH_HEAD` too.

**Pending** (`pending-names-its-commit`). stdout stays the list `check.coverage/pending-report` defines, so the commit line goes to stderr, in `clm-view`'s `text` form. For `--at` the graph and markers are handed to `clm_coverage::pending` and narrowed with `clm_check::pending_for` (`check/pending-narrows-on-a-graph`), the function `pending_in` itself narrows with, so an unknown component gives the same violation either way.

**What the commands decide** (`commands-live-in-their-crates`). `view` states what `clm status` and `clm view` draw for the graph, markers, claim refs, checkout and size report they are handed, and `check.coverage` the lines `clm pending` lists; both crates lie below this one. What only the commands do is this component's: what they read from the repository and never read (`reads-the-repository`), and how they take their arguments and exit (`arguments-and-exit`). `clm-view-repo` is handed the budgets, since it loads no settings, so that `clm status` marks a component by `clm.toml`'s budget is the root's to show (`clm/routes-only`).

**Tests.** Every test of this component sits in `crates/clm-view-repo/tests/` and calls the crate; none runs the `clm` binary.
