---
component: view.repo
base: { contract: "33f2b28e9573f1fa", design: "be68132617d89304" }
---

## MODIFIED Contract
Crate `clm-view-repo`: the `clm status`, `clm view` and `clm pending` commands' reading of the repository, for `view`'s models and renderers (`views-rendered-by-clm`). `status(root, budgets)` and `view(root, name, format, width, source)` load the graph and markers from the working tree (`Source::Tree`) or, for `clm view --at <rev>`, from that commit's tree (`Source::At`, `view-at-rev`), read the checkout, the claim and `propose/*` refs and the time, and return what `view` renders, or the spec graph's violations, or why `<rev>` couldn't be read. `pending(root, component, at)` gives `clm pending [<component>] [--at <rev>]` its commit line and its pending lines (`pending-names-its-commit`). It never fetches and writes no file.

## MODIFIED Design
**At a commit** (`view-at-rev`). `--at <rev>` swaps only where the files come from. The commit's blobs are read from the local objects in memory with `clm_git::tree_blobs` (`vcs/tree-blobs`) rather than extracted as `clm next` extracts a tree, which keeps the cost low enough for the pane's 30-second redraw. They go to the same in-memory calls the `web` Worker uses (`clm_model::load_files`, `clm_check::markers_in`, `clm_check::size_of`), so `--at` marks what `clm view spec` would at that commit. When the CLM root lies below the repository's top, only the paths under it are kept, made relative to it; `linguist_excluded` comes from the commit's `.gitattributes`, and a binary blob, as git judges it, gives no markers. The budgets come from that commit's `clm.toml` alone (`clm_config::node_size_from`): the view describes the commit, so `clm.local.toml` and the environment don't apply.

**`<age>`.** Git has no command that says when a remote was last fetched, and the reflog of `refs/remotes/origin/main` says when the ref last moved and doesn't apply to a `<rev>` that is no ref, so the `FETCH_HEAD` files are read directly. `FETCH_HEAD` is per worktree while `origin/*` is shared by every worktree, so every worktree's file counts (`vcs/common-dir`), including one removed without `git worktree prune`, since it recorded a real fetch of the shared refs, and a fetch of another remote or a `git pull`, since it writes `FETCH_HEAD` too.

**Pending** (`pending-names-its-commit`). stdout stays the list `check.coverage/pending-report` defines, so the commit line goes to stderr, in `clm-view`'s `text` form. For `--at` the graph and markers are handed to `clm_check::pending` and narrowed with `clm_check::pending_for` (`check/pending-narrows-on-a-graph`), the function `pending_in` itself narrows with, so an unknown component gives the same violation either way.

**Tests.** The `--at` tests of the CLI live in `crates/clm/tests/view_at.rs`, so that file counts toward this component and `crates/clm/tests/view.rs` toward `view` alone (`tests-count-where-cited`).
